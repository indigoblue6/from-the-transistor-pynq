module cpu_fault_tb;
    logic clk = 1'b0, reset = 1'b1;
    logic [31:0] instruction_address, instruction_data;
    logic data_valid, data_write, data_byte, data_fault;
    logic [31:0] data_address, data_write_data, data_read_data;
    logic halted, faulted, uart_valid, sim_exit_valid;
    logic [7:0] uart_data;
    logic [31:0] sim_exit_code, debug_pc, debug_instruction, debug_register_data;
    logic [3:0] debug_state, debug_register_index;
    logic debug_register_write;

    always #5 clk = ~clk;
    cpu cpu_i (.*);
    memory_map memory_i (.*);

    task automatic reset_cpu;
        begin
            reset <= 1'b1;
            repeat (3) @(posedge clk);
            reset <= 1'b0;
        end
    endtask

    task automatic expect_fault(input integer limit);
        integer i;
        begin
            for (i = 0; i < limit && !faulted; i = i + 1)
                @(posedge clk);
            if (!faulted)
                $fatal(1, "fault状態へ遷移しませんでした");
        end
    endtask

    initial begin
        #1 memory_i.instruction_memory[0] = 32'hffff_ffff;
        reset_cpu();
        expect_fault(20);

        // movi r1, 0x4001; load r2, [r1 + 0]
        memory_i.instruction_memory[0] = (32'h09 << 26) | (32'd1 << 22) | 32'h4001;
        memory_i.instruction_memory[1] = (32'h0c << 26) | (32'd2 << 22) | (32'd1 << 18);
        reset_cpu();
        expect_fault(40);
        $display("不正命令と未アラインアクセスのRTLテスト成功");
        $finish;
    end
endmodule
