#!/usr/bin/env bash
# Toykit v2 firmware build entry-point.
#
#  ./build.sh             -> release build of all 3 bins
#  ./build.sh debug       -> debug build of all 3 bins
#  ./build.sh check       -> cargo check (fast, no codegen/linking)
#  ./build.sh uf2         -> release build + objcopy + hex-to-uf2
#  ./build.sh release     -> explicit release (same as no arg)
#
# Always sets the three env vars the project needs:
#   CARGO_TARGET_DIR  = C:/build/toykit-target (ASCII short path)
#   LIBCLANG_PATH     = C:\QMK_MSYS\mingw64\bin (libclang.dll location)
#   PATH += QMK_MSYS/mingw64/bin (so libclang.dll's own deps are findable)
#
set -euo pipefail

cd "$(dirname "$0")"

export CARGO_TARGET_DIR=C:/build/toykit-target
export LIBCLANG_PATH='C:\QMK_MSYS\mingw64\bin'
export PATH="/c/QMK_MSYS/mingw64/bin:${PATH}"

MODE="${1:-release}"
case "$MODE" in
  debug|release) CARGO_CMD="build";  PROFILE_FLAG="--${MODE}" ;;
  check)         CARGO_CMD="check";  PROFILE_FLAG="" ;;
  uf2)           CARGO_CMD="build";  PROFILE_FLAG="--release" ;;
  *) echo "usage: $0 [debug|release|check|uf2]"; exit 1 ;;
esac

echo ">>> rustc: $(rustc --version)"
echo ">>> target: ${CARGO_TARGET_DIR}"
echo ">>> profile: ${MODE}"
echo

for BIN in central peripheral peripheral2; do
  echo ">>> ${CARGO_CMD}ing ${BIN} (${MODE})"
  cargo ${CARGO_CMD} ${PROFILE_FLAG} --bin "${BIN}"
done

if [ "${MODE}" = "uf2" ]; then
  echo
  echo ">>> generating hex + uf2"
  RL="${CARGO_TARGET_DIR}/thumbv7em-none-eabihf/release"
  rust-objcopy -O ihex "${RL}/central"     rmk-central.hex
  rust-objcopy -O ihex "${RL}/peripheral"  rmk-peripheral.hex
  rust-objcopy -O ihex "${RL}/peripheral2" rmk-peripheral2.hex
  cargo hex-to-uf2 --input-path rmk-central.hex     --output-path rmk-central.uf2     --family nrf52840
  cargo hex-to-uf2 --input-path rmk-peripheral.hex  --output-path rmk-peripheral.uf2  --family nrf52840
  cargo hex-to-uf2 --input-path rmk-peripheral2.hex --output-path rmk-peripheral2.uf2 --family nrf52840
fi

echo
echo ">>> artifacts:"
ART_DIR="${CARGO_TARGET_DIR}/thumbv7em-none-eabihf/${MODE}"
[ "${MODE}" = "uf2" ] && ART_DIR="${CARGO_TARGET_DIR}/thumbv7em-none-eabihf/release"
ls -la "${ART_DIR}"/{central,peripheral,peripheral2} 2>/dev/null