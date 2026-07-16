#!/bin/sh
set -eu

# build_hardware.tclはhello.memを入力にするため、OSイメージを一時的に同名で渡す。
sh scripts/build_os_image.sh
cp build/indigo-os.mem build/hello.mem
INDIGO_JTAG_ONLY=1 vivado -mode batch -source scripts/build_hardware.tcl -nojournal -nolog
