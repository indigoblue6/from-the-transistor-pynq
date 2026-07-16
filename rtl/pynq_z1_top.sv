module pynq_z1_top (
    inout  wire [53:0] fixed_io_mio,
    inout  wire        fixed_io_ps_clk,
    inout  wire        fixed_io_ps_porb,
    inout  wire        fixed_io_ps_srstb,
    input  logic       clk_125mhz,
    input  logic       reset_button,
    input  logic       uart_rx_pin,
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
    logic uart_rx_available, uart_rx_overrun, uart_rx_interrupt_enable;
    logic jtag_rx_available, jtag_rx_pop;
    logic [7:0] jtag_rx_data;
    (* mark_debug = "true" *) logic [9:0] jtag_host_in_debug;
    (* mark_debug = "true" *) logic [8:0] jtag_host_out_debug;
    logic uart_rx_interrupt_request, uart_rx_pop, uart_rx_control_write;
    logic [7:0] uart_rx_data;
    logic [31:0] uart_rx_control_data;
    (* mark_debug = "true" *) logic halted_internal;
    (* mark_debug = "true" *) logic faulted_internal;
    (* mark_debug = "true" *) logic hardware_done;
    (* mark_debug = "true" *) logic [135:0] uart_history = 136'b0;
    (* mark_debug = "true" *) logic current_privileged;
    (* mark_debug = "true" *) logic [2:0] interrupt_pending_debug;
    (* mark_debug = "true" *) logic [2:0] interrupt_enable_debug;
    (* mark_debug = "true" *) logic [31:0] epc_debug, cause_debug, badaddr_debug;
    (* mark_debug = "true" *) logic [31:0] timer_count_debug;
    (* mark_debug = "true" *) logic interrupt_taken_debug;
    logic uart_ready;
    logic uart_fifo_input_ready, uart_fifo_valid;
    logic [7:0] uart_fifo_data;
    logic prog_uart_fifo_input_ready, prog_uart_fifo_valid, prog_uart_fifo_ready;
    logic [7:0] prog_uart_fifo_data, prog_uart_mailbox_data;
    logic prog_uart_mailbox_pending, prog_uart_mailbox_toggle;
    logic [1:0] prog_uart_ack_sync;
    logic [63:0] ps_gpio_input, ps_gpio_output, ps_gpio_tri;
    logic prog_uart_rx_available, prog_uart_rx_pop, prog_uart_rx_ack;
    logic physical_uart_rx_pop;
    logic [1:0] prog_uart_rx_toggle_sync;
    logic [7:0] prog_uart_rx_data;
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
    assign hardware_done = halted_internal && !uart_fifo_valid && uart_ready &&
        !prog_uart_fifo_valid && !prog_uart_mailbox_pending;
    assign led = {heartbeat[24], uart_ready, faulted_internal, halted_internal};

    // EMIO GPIO 0～7は文字、8は送信トグル、9はPSからのackとして使う。
    // 16～24はPSからの受信文字とトグル、25はPLからのackとして使う。
    assign ps_gpio_input = {38'b0, prog_uart_rx_ack, 16'b0,
        prog_uart_mailbox_toggle, prog_uart_mailbox_data};
    assign prog_uart_fifo_ready = prog_uart_mailbox_pending &&
        (prog_uart_ack_sync[1] == prog_uart_mailbox_toggle);

    always_ff @(posedge cpu_clk) begin
        if (reset) begin
            prog_uart_ack_sync <= 2'b0;
            prog_uart_mailbox_data <= 8'b0;
            prog_uart_mailbox_pending <= 1'b0;
            prog_uart_mailbox_toggle <= 1'b0;
        end else begin
            prog_uart_ack_sync <= {prog_uart_ack_sync[0], ps_gpio_output[9]};
            if (!prog_uart_mailbox_pending && prog_uart_fifo_valid) begin
                prog_uart_mailbox_data <= prog_uart_fifo_data;
                prog_uart_mailbox_toggle <= !prog_uart_mailbox_toggle;
                prog_uart_mailbox_pending <= 1'b1;
            end else if (prog_uart_fifo_ready) begin
                prog_uart_mailbox_pending <= 1'b0;
            end
        end
    end

    always_ff @(posedge cpu_clk) begin
        if (reset) begin
            prog_uart_rx_toggle_sync <= 2'b0;
            prog_uart_rx_data <= 8'b0;
            prog_uart_rx_available <= 1'b0;
            prog_uart_rx_ack <= 1'b0;
        end else begin
            prog_uart_rx_toggle_sync <= {
                prog_uart_rx_toggle_sync[0], ps_gpio_output[24]};
            if (!prog_uart_rx_available &&
                    prog_uart_rx_toggle_sync[1] != prog_uart_rx_ack) begin
                prog_uart_rx_data <= ps_gpio_output[23:16];
                prog_uart_rx_available <= 1'b1;
            end else if (prog_uart_rx_pop) begin
                prog_uart_rx_available <= 1'b0;
                prog_uart_rx_ack <= prog_uart_rx_toggle_sync[1];
            end
        end
    end

    processing_system7_0 ps_i (
        .MIO(fixed_io_mio), .PS_CLK(fixed_io_ps_clk),
        .PS_PORB(fixed_io_ps_porb), .PS_SRSTB(fixed_io_ps_srstb),
        .GPIO_I(ps_gpio_input), .GPIO_O(ps_gpio_output), .GPIO_T(ps_gpio_tri)
    );

    cpu cpu_i (
        .clk(cpu_clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .uart_rx_pending(uart_rx_interrupt_request || jtag_rx_available ||
            prog_uart_rx_available),
        .halted(halted_internal), .faulted(faulted_internal),
        .debug_pc(unused_debug_pc), .debug_instruction(unused_debug_instruction),
        .debug_state(unused_debug_state), .debug_register_write(unused_debug_register_write),
        .debug_register_index(unused_debug_register_index), .debug_register_data(unused_debug_register_data),
        .debug_privileged(current_privileged),
        .debug_interrupt_pending(interrupt_pending_debug),
        .debug_interrupt_enable(interrupt_enable_debug), .debug_epc(epc_debug),
        .debug_cause(cause_debug), .debug_badaddr(badaddr_debug),
        .debug_timer_count(timer_count_debug), .debug_interrupt_taken(interrupt_taken_debug)
    );
    vio_0 vio_i (
        .clk(cpu_clk), .probe_in0(jtag_host_out_debug), .probe_out0(jtag_host_in_debug)
    );
    memory_map #(.INSTRUCTION_INIT_FILE("hello.mem")) memory_i (
        .clk(cpu_clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .uart_rx_available(jtag_rx_available || prog_uart_rx_available ||
            uart_rx_available), .uart_rx_overrun(uart_rx_overrun),
        .uart_rx_interrupt_enable(uart_rx_interrupt_enable),
        .uart_rx_data(jtag_rx_available ? jtag_rx_data :
            (prog_uart_rx_available ? prog_uart_rx_data : uart_rx_data)),
        .uart_rx_pop(uart_rx_pop), .uart_rx_control_write(uart_rx_control_write),
        .uart_rx_control_data(uart_rx_control_data),
        .uart_valid(uart_valid), .uart_data(uart_data),
        .sim_exit_valid(sim_exit_valid), .sim_exit_code(sim_exit_code)
    );
    assign jtag_rx_pop = uart_rx_pop && jtag_rx_available;
    assign prog_uart_rx_pop = uart_rx_pop && !jtag_rx_available &&
        prog_uart_rx_available;
    assign physical_uart_rx_pop = uart_rx_pop && !jtag_rx_available &&
        !prog_uart_rx_available;
    jtag_console #(.FIFO_DEPTH(16)) jtag_console_i (
        .clk(cpu_clk), .reset(reset), .host_in(jtag_host_in_debug), .host_out(jtag_host_out_debug),
        .tx_push(uart_fifo_valid && uart_ready), .tx_data(uart_fifo_data),
        .rx_available(jtag_rx_available), .rx_data(jtag_rx_data), .rx_pop(jtag_rx_pop)
    );
    uart_rx #(.CLOCKS_PER_BIT(271), .FIFO_DEPTH(16)) uart_rx_i (
        .clk(cpu_clk), .reset(reset), .rx(uart_rx_pin), .pop(physical_uart_rx_pop),
        .control_write(uart_rx_control_write), .control_data(uart_rx_control_data),
        .data(uart_rx_data), .available(uart_rx_available), .overrun(uart_rx_overrun),
        .interrupt_enable(uart_rx_interrupt_enable),
        .interrupt_request(uart_rx_interrupt_request)
    );
    uart_fifo uart_fifo_i (
        .clk(cpu_clk), .reset(reset), .input_valid(uart_valid), .input_data(uart_data),
        .input_ready(uart_fifo_input_ready), .output_valid(uart_fifo_valid),
        .output_data(uart_fifo_data), .output_ready(uart_ready)
    );
    uart_fifo #(.DEPTH_LOG2(8)) prog_uart_fifo_i (
        .clk(cpu_clk), .reset(reset), .input_valid(uart_valid), .input_data(uart_data),
        .input_ready(prog_uart_fifo_input_ready), .output_valid(prog_uart_fifo_valid),
        .output_data(prog_uart_fifo_data), .output_ready(prog_uart_fifo_ready)
    );
    uart_tx #(.CLOCKS_PER_BIT(271)) uart_tx_i (
        .clk(cpu_clk), .reset(reset), .valid(uart_fifo_valid), .data(uart_fifo_data),
        .ready(uart_ready), .tx(uart_tx_pin)
    );
endmodule
