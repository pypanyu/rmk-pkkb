#![no_std]
#![no_main]

// ─────────────────────────────────────────────────────────────
// Totem USB dongle 固件（第三块 nRF52840 板）
// 角色：USB 插电脑，BLE 只与一把键盘（分体 central）通信。
// 无矩阵、无键位表；只保存"绑定了哪把键盘"。
// HID 报文 + Vial/Rynk 配置协议原样透传。
// 参照官方示例 examples/use_rust/nrf_dongle（nRF54 版）移植到 nRF52840。
// ─────────────────────────────────────────────────────────────

use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::{RNG, USBD};
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;
use embassy_nrf::usb::{self, Driver};
use embassy_nrf::{bind_interrupts, pac, rng};
use nrf_mpsl::Flash;
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, mpsl};
use panic_probe as _;
use rmk::config::{DeviceConfig, StorageConfig};
use rmk::dongle::{Dongle, DongleRouter};
use rmk::storage::new_storage_without_keymap;
use rmk::usb::UsbTransport;
use rmk::{DefaultPacketPool, PacketPool, run_all};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<USBD>;
    RNG => rng::InterruptHandler<RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler, usb::vbus_detect::InterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

/// nrf-sdc 协议栈内存池大小（字节数组长度）
const SDC_MEM_SIZE: usize = 5960;

/// 每条链路的 L2CAP 收/发缓冲数（一把键盘，一条链路，4/4 足够）
const L2CAP_TXQ: u8 = 4;
const L2CAP_RXQ: u8 = 4;

/// 纯 central 控制器：dongle 只发起连接和扫描，从不广播。
fn build_sdc<'d, const N: usize>(
    p: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    mem: &'d mut sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    sdc::Builder::new()?
        .support_scan()
        .support_central()
        .support_dle_central()
        .support_phy_update_central()
        .support_le_2m_phy()
        // 一把键盘，一条连接
        .central_count(1)?
        .buffer_cfg(
            DefaultPacketPool::MTU as u16,
            DefaultPacketPool::MTU as u16,
            L2CAP_TXQ,
            L2CAP_RXQ,
        )?
        .build(p, rng, mpsl, mem)
}

/// BLE 地址取自芯片出厂 FICR 设备 ID，置上静态地址标志位
fn ble_addr() -> [u8; 6] {
    let ficr = pac::FICR;
    let high = u64::from(ficr.deviceid(1).read());
    let addr = high << 32 | u64::from(ficr.deviceid(0).read());
    let addr = addr | 0x0000_c000_0000_0000;
    unwrap!(addr.to_le_bytes()[..6].try_into())
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Totem dongle on nRF52840");

    // 板级初始化：启用 DCDC 稳压器（USB 供电下更省功耗、更稳）
    let mut nrf_config = embassy_nrf::config::Config::default();
    nrf_config.dcdc.reg0_voltage = Some(embassy_nrf::config::Reg0Voltage::_3V3);
    nrf_config.dcdc.reg0 = true;
    nrf_config.dcdc.reg1 = true;
    let p = embassy_nrf::init(nrf_config);

    // ── MPSL（多协议服务层，nrf-sdc 的运行时）──
    let mpsl_p = mpsl::Peripherals::new(p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31);
    let lfclk_cfg = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_CTIV as u8,
        rc_temp_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_TEMP_CTIV as u8,
        accuracy_ppm: 500,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    static SESSION_MEM: StaticCell<mpsl::SessionMem<1>> = StaticCell::new();
    let mpsl = MPSL.init(unwrap!(mpsl::MultiprotocolServiceLayer::with_timeslots(
        mpsl_p,
        Irqs,
        lfclk_cfg,
        SESSION_MEM.init(mpsl::SessionMem::new())
    )));
    spawner.spawn(unwrap!(mpsl_task(&*mpsl)));
    info!("MPSL started");

    // ── BLE 控制器（central-only）──
    let sdc_p = sdc::Peripherals::new(
        p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24, p.PPI_CH25,
        p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29,
    );
    let mut rng = rng::Rng::new(p.RNG, Irqs);
    let mut sdc_mem = sdc::Mem::<SDC_MEM_SIZE>::new();
    let sdc = unwrap!(build_sdc(sdc_p, &mut rng, mpsl, &mut sdc_mem));
    info!("SDC built (central only)");

    // ── USB（nRF52840 USBD 自带 VBUS 硬件检测，无需外部电路）──
    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));

    // USB 描述符：PID 与键盘区分开（0x4643 键盘 / 0x4644 dongle）
    let device_config = DeviceConfig {
        vid: 0x4c4b,
        pid: 0x4644,
        manufacturer: "Totem",
        product_name: "Totem Dongle",
        ..DeviceConfig::default()
    };

    // ── 存储（只存"绑定了哪把键盘"，不存键位）──
    // start_addr = 0 时 nRF BLE 芯片默认落在 0x60000；给 6 个扇区余量
    let storage_config = StorageConfig {
        num_sectors: 6,
        ..Default::default()
    };
    let flash = Flash::take(mpsl, p.NVMC);
    let mut storage = new_storage_without_keymap(flash, storage_config).await;
    info!("Storage initialized");

    // ── dongle 本体 + USB 透传，由共享 router 串联 ──
    // router 被_BLE 侧（与键盘的链路）和 USB host 会话两侧共用，报文原样转发。
    let router = DongleRouter::new();
    let mut dongle = Dongle::new(sdc, ble_addr(), &router);
    let mut usb_transport = UsbTransport::new(driver, device_config).with_dongle_router(&router);

    run_all!(usb_transport, dongle, storage).await;
}
