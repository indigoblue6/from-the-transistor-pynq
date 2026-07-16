PYTHON ?= python3
PROGRAM ?= hello
BUILD_DIR := build
PORT ?= /dev/ttyUSB1
ASSEMBLER := assembler/assembler.py

.PHONY: all test test-assembler test-emulator test-rtl test-exceptions test-interrupts assemble emulate simulate compiler test-compiler test-lexer test-parser test-semantic test-codegen test-c-integration compile run-c simulate-c test-c-rtl os kernel user-programs os-image run-os simulate-os test-os test-scheduler test-syscalls test-memory test-ramfs test-shell test-os-differential ps-uart-bridge hardware-build hardware-test hardware-uart-test hardware-c-build hardware-c-test hardware-os-test hardware-os-uart-test jtag-console test-all clean

all: test

test: test-assembler test-emulator test-rtl

test-assembler:
	$(PYTHON) -m pytest -q assembler/tests

test-emulator:
	cargo test --manifest-path emulator/Cargo.toml
	sh scripts/test_examples.sh

test-rtl:
	$(PYTHON) $(ASSEMBLER) sim/programs/rtl_test.s -o $(BUILD_DIR)/rtl_test.bin --mem $(BUILD_DIR)/rtl_test.mem
	$(PYTHON) $(ASSEMBLER) examples/hello.s -o $(BUILD_DIR)/hello.bin --mem $(BUILD_DIR)/hello.mem
	sh sim/run_sim.sh rtl_test
	sh sim/run_sim.sh hello
	sh sim/run_fault_tests.sh
	sh sim/run_uart_fifo_test.sh

test-exceptions:
	sh sim/run_exception_tests.sh

test-interrupts: test-exceptions
	sh sim/run_uart_rx_test.sh

assemble:
	mkdir -p $(BUILD_DIR)
	$(PYTHON) $(ASSEMBLER) examples/$(PROGRAM).s -o $(BUILD_DIR)/$(PROGRAM).bin --mem $(BUILD_DIR)/$(PROGRAM).mem

emulate: assemble
	cargo run --quiet --manifest-path emulator/Cargo.toml -- $(BUILD_DIR)/$(PROGRAM).bin

simulate: assemble
	sh sim/run_sim.sh $(PROGRAM)

hardware-build: PROGRAM=hello
hardware-build: assemble
	vivado -mode batch -source scripts/build_hardware.tcl -nojournal -nolog

ps-uart-bridge:
	sh scripts/build_ps_uart_bridge.sh

hardware-test: hardware-build ps-uart-bridge
	@$(PYTHON) scripts/verify_usb_uart.py "$(PORT)" --timeout 30 & receiver=$$!; \
	  sleep 1; \
	  xsct scripts/program_prog_uart.tcl; \
	  INDIGO_SKIP_PROGRAM=1 vivado -mode batch -source scripts/program_hardware.tcl -nojournal -nolog; \
	  $(PYTHON) scripts/verify_hardware_capture.py; \
	  wait $$receiver

hardware-uart-test: hardware-build
	@test -n "$(PORT)" || (echo "PORT=/dev/ttyUSBx を指定してください" >&2; exit 2)
	@$(PYTHON) scripts/verify_usb_uart.py "$(PORT)" --timeout 30 & receiver=$$!; \
	  sleep 1; \
	  vivado -mode batch -source scripts/program_hardware.tcl -nojournal -nolog; \
	  $(PYTHON) scripts/verify_hardware_capture.py; \
	  wait $$receiver

compiler:
	cargo build --manifest-path compiler/Cargo.toml

test-compiler:
	cargo test --manifest-path compiler/Cargo.toml
	sh scripts/test_c_negative.sh

test-lexer:
	cargo test --manifest-path compiler/Cargo.toml lexer

test-parser:
	cargo test --manifest-path compiler/Cargo.toml parser

test-semantic:
	cargo test --manifest-path compiler/Cargo.toml semantic

test-codegen:
	cargo test --manifest-path compiler/Cargo.toml --test codegen

test-c-integration: test-compiler
	sh scripts/test_c_integration.sh

compile: compiler
	mkdir -p $(BUILD_DIR)
	cargo run --quiet --manifest-path compiler/Cargo.toml -- examples/c/$(PROGRAM).pc -o $(BUILD_DIR)/$(PROGRAM).s
	$(PYTHON) $(ASSEMBLER) $(BUILD_DIR)/$(PROGRAM).s -o $(BUILD_DIR)/$(PROGRAM).bin --mem $(BUILD_DIR)/$(PROGRAM).mem

run-c: compile
	cargo run --quiet --manifest-path emulator/Cargo.toml -- $(BUILD_DIR)/$(PROGRAM).bin --max-steps 5000000

simulate-c: compile
	SKIP_EXPECT=1 MAX_CYCLES=5000000 sh sim/run_sim.sh $(PROGRAM)

test-c-rtl:
	sh scripts/test_c_rtl.sh

os kernel os-image: compiler
	sh scripts/build_os_image.sh

user-programs:
	@echo "ユーザーモード実行基盤は未実装です" >&2
	@exit 2

run-os: os-image
	sh scripts/test_os_emulator.sh

simulate-os: os-image
	sh scripts/test_os_rtl.sh

test-scheduler:
	@echo "複数タスクのスケジューラは未実装です" >&2
	@exit 2

test-syscalls:
	@echo "ユーザー向けシステムコールは未実装です" >&2
	@exit 2

test-memory: os-image
	sh scripts/test_os_emulator.sh --allocator-only

test-ramfs: os-image
	sh scripts/test_os_emulator.sh --ramfs-only

test-shell: os-image
	sh scripts/test_os_emulator.sh --shell-only

test-os: test-exceptions test-interrupts test-memory test-ramfs test-shell

test-os-differential: os-image
	sh scripts/test_os_differential.sh

hardware-os-test:
	sh scripts/build_os_hardware.sh
	sh scripts/build_ps_uart_bridge.sh
	xsct scripts/program_prog_uart.tcl
	@echo "IndigoOSを書き込みました。picocom -b 115200 /dev/ttyUSB1 でPROG UARTシェルを使用できます"

hardware-os-uart-test: hardware-os-test
	@echo "IndigoOSのPROG UARTシェルを起動しました"

jtag-console:
	vivado -mode tcl -source scripts/jtag_console.tcl

test-all: test test-c-integration test-c-rtl test-os-differential

hardware-c-build:
	$(MAKE) compile PROGRAM=hardware
	cp $(BUILD_DIR)/hardware.mem $(BUILD_DIR)/hello.mem
	vivado -mode batch -source scripts/build_hardware.tcl -nojournal -nolog

hardware-c-test: hardware-c-build
	vivado -mode batch -source scripts/program_hardware.tcl -nojournal -nolog
	$(PYTHON) scripts/verify_hardware_capture.py

clean:
	rm -rf $(BUILD_DIR) emulator/target compiler/target .pytest_cache assembler/__pycache__ assembler/tests/__pycache__
