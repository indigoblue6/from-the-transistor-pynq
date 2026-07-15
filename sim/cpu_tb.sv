module cpu_tb;
    logic clk = 1'b0;
    logic reset = 1'b1;
    logic [31:0] instruction_address, instruction_data;
    logic data_valid, data_write, data_byte;
    logic [31:0] data_address, data_write_data, data_read_data;
    logic data_fault, halted, faulted;
    logic uart_valid, sim_exit_valid;
    logic [7:0] uart_data;
    logic [31:0] sim_exit_code;
    logic [31:0] debug_pc, debug_instruction, debug_register_data;
    logic [3:0] debug_state, debug_register_index;
    logic debug_register_write;
    string memory_file;
    string expected;
    string output_file;
    string register_file;
    string actual = "";
    integer cycles, max_cycles, output_descriptor, register_descriptor, register_index;
    logic [31:0] register_shadow [0:15];

    always #5 clk = ~clk;

    cpu cpu_i (.*);
    memory_map memory_i (
        .clk(clk), .reset(reset),
        .instruction_address(instruction_address), .instruction_data(instruction_data),
        .data_valid(data_valid), .data_write(data_write), .data_byte(data_byte),
        .data_address(data_address), .data_write_data(data_write_data),
        .data_read_data(data_read_data), .data_fault(data_fault),
        .uart_valid(uart_valid), .uart_data(uart_data),
        .sim_exit_valid(sim_exit_valid), .sim_exit_code(sim_exit_code)
    );

    initial begin
        if (!$value$plusargs("MEM=%s", memory_file))
            $fatal(1, "MEM引数がありません");
        if (!$value$plusargs("EXPECT=%s", expected))
            expected = "";
        if (!$value$plusargs("MAX_CYCLES=%d", max_cycles))
            max_cycles = 2000000;
        output_descriptor = 0;
        if ($value$plusargs("OUTPUT_FILE=%s", output_file)) begin
            output_descriptor = $fopen(output_file, "wb");
            if (output_descriptor == 0)
                $fatal(1, "出力ファイルを開けません: %s", output_file);
        end
        for (register_index = 0; register_index < 16; register_index = register_index + 1)
            register_shadow[register_index] = 32'b0;
        register_shadow[15] = 32'h0001_0000;
        register_descriptor = 0;
        if ($value$plusargs("REGISTER_FILE=%s", register_file)) begin
            register_descriptor = $fopen(register_file, "w");
            if (register_descriptor == 0)
                $fatal(1, "レジスタファイルを開けません: %s", register_file);
        end
        #1 $readmemh(memory_file, memory_i.instruction_memory);
        repeat (3) @(posedge clk);
        reset <= 1'b0;
        for (cycles = 0; cycles < max_cycles; cycles = cycles + 1) begin
            @(posedge clk);
            if (debug_register_write && debug_register_index != 0)
                register_shadow[debug_register_index] = debug_register_data;
            if (uart_valid) begin
                actual = {actual, uart_data};
                $write("%c", uart_data);
                if (output_descriptor != 0)
                    $fwrite(output_descriptor, "%c", uart_data);
            end
            if (faulted)
                $fatal(1, "CPU fault: PC=%08x IR=%08x state=%0d", debug_pc, debug_instruction, debug_state);
            if (sim_exit_valid && sim_exit_code != 0)
                $fatal(1, "SIM_EXIT失敗: %0d", sim_exit_code);
            if (halted || sim_exit_valid) begin
                if (register_descriptor != 0) begin
                    for (register_index = 0; register_index < 16; register_index = register_index + 1)
                        $fwrite(register_descriptor, "%08x\n", register_shadow[register_index]);
                    $fclose(register_descriptor);
                end
                if (output_descriptor != 0)
                    $fclose(output_descriptor);
                if (expected != "" && actual != expected)
                    $fatal(1, "出力不一致: actual='%s' expected='%s'", actual, expected);
                $display("RTLテスト成功（%0d cycles）", cycles);
                $finish;
            end
        end
        $fatal(1, "タイムアウト: PC=%08x IR=%08x state=%0d", debug_pc, debug_instruction, debug_state);
    end
endmodule
