module pynq_z1_top (
    input  logic       clk_125mhz,
    input  logic       reset_button,
    output logic [3:0] led,
    output logic       uart_tx_pin
);
    logic [7:0] reset_counter = 8'hff;
    logic reset;
    logic [31:0] instruction_address, instruction_data;
    logic data_valid, data_write, data_byte, data_fault;
    logic [31:0] data_address, data_write_data, data_read_data;
    logic uart_valid, sim_exit_valid;
    logic [7:0] uart_data;
    logic [31:0] sim_exit_code;
    (* mark_debug = "true" *) logic halted_internal;
    (* mark_debug = "true" *) logic faulted_internal;
    (* mark_debug = "true" *) logic hardware_done;
    (* mark_debug = "true" *) logic [135:0] uart_history = 136'b0;
    logic uart_ready;
    logic uart_fifo_input_ready, uart_fifo_valid;
    logic [7:0] uart_fifo_data;
    logic [31:0] heartbeat = 32'b0;
    logic [31:0] unused_debug_pc, unused_debug_instruction, unused_debug_register_data;
    logic [3:0] unused_debug_state, unused_debug_register_index;
    logic unused_debug_register_write;
    logic cpu_clk, cpu_clk_unbuffered, mmcm_feedback, mmcm_locked;

    // 7-series対応MMCMで125 MHzから31.25 MHzを生成する。
    MMCME2_BASE #(
        .CLKIN1_PERIOD(8.0), .CLKFBOUT_MULT_F(8.0),
        .DIVCLK_DIVIDE(1), .CLKOUT0_DIVIDE_F(32.0)
    ) cpu_mmcm (
        .CLKIN1(clk_125mhz), .CLKFBIN(mmcm_feedback),
        .CLKFBOUT(mmcm_feedback), .CLKOUT0(cpu_clk_unbuffered),
        .LOCKED(mmcm_locked), .PWRDWN(1'b0), .RST(reset_button)
    );
    BUFG cpu_clock_buffer (.I(cpu_clk_unbuffered), .O(cpu_clk));

    always_ff @(posedge clk_125mhz) begin
        if (reset_button)
            reset_counter <= 8'hff;
        else if (reset_counter != 0)
            reset_counter <= reset_counter - 1'b1;
    end
    always_ff @(posedge cpu_clk) begin
        heartbeat <= heartbeat + 1'b1;
        if (uart_fifo_valid && uart_ready)
            uart_history <= {uart_history[127:0], uart_fifo_data};
    end
    assign reset = reset_button || !mmcm_locked || (reset_counter != 0);
    assign hardware_done = halted_internal && !uart_fifo_valid && uart_ready;
    assign led = {heartbeat[24], uart_ready, faulted_internal, halted_internal};

    cpu cpu_i (
        .clk(cpu_clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .halted(halted_internal), .faulted(faulted_internal),
        .debug_pc(unused_debug_pc), .debug_instruction(unused_debug_instruction),
        .debug_state(unused_debug_state), .debug_register_write(unused_debug_register_write),
        .debug_register_index(unused_debug_register_index), .debug_register_data(unused_debug_register_data)
    );
    memory_map #(.INSTRUCTION_INIT_FILE("hello.mem")) memory_i (
        .clk(cpu_clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .uart_valid(uart_valid), .uart_data(uart_data),
        .sim_exit_valid(sim_exit_valid), .sim_exit_code(sim_exit_code)
    );
    uart_fifo uart_fifo_i (
        .clk(cpu_clk), .reset(reset), .input_valid(uart_valid), .input_data(uart_data),
        .input_ready(uart_fifo_input_ready), .output_valid(uart_fifo_valid),
        .output_data(uart_fifo_data), .output_ready(uart_ready)
    );
    uart_tx #(.CLOCKS_PER_BIT(271)) uart_tx_i (
        .clk(cpu_clk), .reset(reset), .valid(uart_fifo_valid), .data(uart_fifo_data),
        .ready(uart_ready), .tx(uart_tx_pin)
    );
endmodule
