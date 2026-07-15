PYTHON ?= python3
PROGRAM ?= hello
BUILD_DIR := build
PORT ?=
ASSEMBLER := assembler/assembler.py

.PHONY: all test test-assembler test-emulator test-rtl assemble emulate simulate compiler test-compiler test-lexer test-parser test-semantic test-codegen test-c-integration compile run-c simulate-c test-c-rtl test-all hardware-build hardware-test hardware-uart-test hardware-c-build hardware-c-test clean

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

hardware-test: hardware-build
	vivado -mode batch -source scripts/program_hardware.tcl -nojournal -nolog
	$(PYTHON) scripts/verify_hardware_capture.py

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

test-all: test test-c-integration test-c-rtl

hardware-c-build:
	$(MAKE) compile PROGRAM=hardware
	cp $(BUILD_DIR)/hardware.mem $(BUILD_DIR)/hello.mem
	vivado -mode batch -source scripts/build_hardware.tcl -nojournal -nolog

hardware-c-test: hardware-c-build
	vivado -mode batch -source scripts/program_hardware.tcl -nojournal -nolog
	$(PYTHON) scripts/verify_hardware_capture.py

clean:
	rm -rf $(BUILD_DIR) emulator/target compiler/target .pytest_cache assembler/__pycache__ assembler/tests/__pycache__
