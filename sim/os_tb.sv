module os_tb;
    localparam integer UART_CLOCKS_PER_BIT = 271;
    logic clk = 1'b0, reset = 1'b1, serial_rx = 1'b1;
    logic [31:0] instruction_address, instruction_data;
    logic data_valid, data_write, data_byte, data_fault;
    logic [31:0] data_address, data_write_data, data_read_data;
    logic halted, faulted, uart_valid, sim_exit_valid;
    logic [7:0] uart_data;
    logic [31:0] sim_exit_code;
    logic uart_rx_available, uart_rx_overrun, uart_rx_interrupt_enable;
    logic uart_rx_interrupt_request, uart_rx_pop, uart_rx_control_write;
    logic [7:0] uart_rx_data;
    logic [31:0] uart_rx_control_data;
    logic [31:0] debug_pc, debug_instruction, debug_register_data;
    logic [3:0] debug_state, debug_register_index;
    logic debug_register_write, debug_privileged, debug_interrupt_taken;
    logic debug_trap_valid, debug_timer_interrupt, debug_external_interrupt;
    logic debug_unrecoverable_fault, external_irq = 1'b0;
    logic [2:0] debug_interrupt_pending, debug_interrupt_enable;
    logic [31:0] debug_epc, debug_cause, debug_badaddr, debug_timer_count;
    logic [31:0] debug_kernel_sp;
    string memory_file, input_file, output_file;
    integer input_descriptor, output_descriptor, character, cycles;

    always #5 clk = ~clk;

    cpu cpu_i (
        .clk(clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .uart_rx_pending(uart_rx_interrupt_request), .external_irq(external_irq),
        .halted(halted), .faulted(faulted),
        .debug_pc(debug_pc), .debug_instruction(debug_instruction), .debug_state(debug_state),
        .debug_register_write(debug_register_write), .debug_register_index(debug_register_index),
        .debug_register_data(debug_register_data), .debug_privileged(debug_privileged),
        .debug_interrupt_pending(debug_interrupt_pending),
        .debug_interrupt_enable(debug_interrupt_enable), .debug_epc(debug_epc),
        .debug_cause(debug_cause), .debug_badaddr(debug_badaddr),
        .debug_timer_count(debug_timer_count), .debug_interrupt_taken(debug_interrupt_taken),
        .debug_trap_valid(debug_trap_valid), .debug_timer_interrupt(debug_timer_interrupt),
        .debug_external_interrupt(debug_external_interrupt),
        .debug_unrecoverable_fault(debug_unrecoverable_fault),
        .debug_kernel_sp(debug_kernel_sp)
    );

    memory_map memory_i (
        .clk(clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .uart_valid(uart_valid), .uart_data(uart_data),
        .uart_rx_available(uart_rx_available), .uart_rx_overrun(uart_rx_overrun),
        .uart_rx_interrupt_enable(uart_rx_interrupt_enable), .uart_rx_data(uart_rx_data),
        .uart_rx_pop(uart_rx_pop), .uart_rx_control_write(uart_rx_control_write),
        .uart_rx_control_data(uart_rx_control_data),
        .sim_exit_valid(sim_exit_valid), .sim_exit_code(sim_exit_code)
    );

    uart_rx #(.CLOCKS_PER_BIT(UART_CLOCKS_PER_BIT), .FIFO_DEPTH(16)) uart_rx_i (
        .clk(clk), .reset(reset), .rx(serial_rx), .pop(uart_rx_pop),
        .control_write(uart_rx_control_write), .control_data(uart_rx_control_data),
        .data(uart_rx_data), .available(uart_rx_available), .overrun(uart_rx_overrun),
        .interrupt_enable(uart_rx_interrupt_enable),
        .interrupt_request(uart_rx_interrupt_request)
    );

    task automatic send_byte(input logic [7:0] value);
        integer bit_number;
        begin
            serial_rx = 1'b0;
            repeat (UART_CLOCKS_PER_BIT) @(posedge clk);
            for (bit_number = 0; bit_number < 8; bit_number = bit_number + 1) begin
                serial_rx = value[bit_number];
                repeat (UART_CLOCKS_PER_BIT) @(posedge clk);
            end
            serial_rx = 1'b1;
            repeat (UART_CLOCKS_PER_BIT) @(posedge clk);
        end
    endtask

    initial begin
        wait (!reset);
        repeat (2000) @(posedge clk);
        while (!$feof(input_descriptor)) begin
            character = $fgetc(input_descriptor);
            if (character >= 0)
                send_byte(character[7:0]);
        end
        $fclose(input_descriptor);
    end

    initial begin
        if (!$value$plusargs("MEM=%s", memory_file) ||
            !$value$plusargs("INPUT=%s", input_file) ||
            !$value$plusargs("OUTPUT=%s", output_file))
            $fatal(1, "MEM/INPUT/OUTPUT引数が必要です");
        output_descriptor = $fopen(output_file, "wb");
        input_descriptor = $fopen(input_file, "rb");
        if (output_descriptor == 0 || input_descriptor == 0)
            $fatal(1, "OSテストファイルを開けません");
        #1 $readmemh(memory_file, memory_i.instruction_memory);
        repeat (4) @(posedge clk);
        reset = 1'b0;
        for (cycles = 0; cycles < 5000000; cycles = cycles + 1) begin
            @(posedge clk);
            if (uart_valid) begin
                $write("%c", uart_data);
                $fwrite(output_descriptor, "%c", uart_data);
            end
            if (faulted)
                $fatal(1, "OSがterminal fault: pc=%08x cause=%0d badaddr=%08x",
                    debug_pc, debug_cause, debug_badaddr);
            if (uart_rx_overrun)
                $fatal(1, "OS入力でUART RX overrunが発生しました");
            if (sim_exit_valid) begin
                $fclose(output_descriptor);
                if (sim_exit_code != 0)
                    $fatal(1, "OS SIM_EXIT失敗: %0d", sim_exit_code);
                $display("RTL IndigoOSシェルテスト成功（%0d cycles, tick=%0d）",
                    cycles, debug_timer_count);
                $finish;
            end
        end
        $fatal(1, "OSテストタイムアウト: pc=%08x cause=%0d", debug_pc, debug_cause);
    end
endmodule
