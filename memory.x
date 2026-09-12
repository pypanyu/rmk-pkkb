MEMORY
{
  /* NOTE 1 K = 1 KiB = 1024 bytes */
  /* 当前配置：带 Adafruit nRF52 bootloader 的板子（nice!nano 等，支持 UF2 拖拽烧录） */
  FLASH : ORIGIN = 0x00001000, LENGTH = 1020K
  RAM : ORIGIN = 0x20000008, LENGTH = 255K

  /* 若你的板子没有 bootloader（裸片 + 调试探针烧录），改用下面这组：
  FLASH : ORIGIN = 0x00000000, LENGTH = 1024K
  RAM : ORIGIN = 0x20000000, LENGTH = 256K
  */
}
