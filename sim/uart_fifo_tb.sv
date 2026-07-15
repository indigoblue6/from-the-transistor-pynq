module uart_fifo_tb;
    logic clk = 0, reset = 1;
    logic input_valid = 0, input_ready;
    logic [7:0] input_data = 0, output_data;
    logic output_valid, output_ready = 0;
    integer cycle = 0, consumed = 0, i;
    byte expected [0:16];

    always #5 clk = ~clk;
    uart_fifo fifo_i (.*);

    always @(posedge clk) begin
        cycle <= cycle + 1;
        output_ready <= (cycle % 7 == 0);
        if (output_valid && output_ready) begin
            if (output_data !== expected[consumed])
                $fatal(1, "FIFO順序不一致: index=%0d actual=%02x expected=%02x", consumed, output_data, expected[consumed]);
            consumed <= consumed + 1;
        end
    end

    initial begin
        expected[0] = "H"; expected[1] = "e"; expected[2] = "l"; expected[3] = "l";
        expected[4] = "o"; expected[5] = ","; expected[6] = " "; expected[7] = "P";
        expected[8] = "Y"; expected[9] = "N"; expected[10] = "Q"; expected[11] = " ";
        expected[12] = "C"; expected[13] = "P"; expected[14] = "U"; expected[15] = "!";
        expected[16] = 8'h0a;
        repeat (3) @(posedge clk);
        reset = 0;
        for (i = 0; i < 17; i = i + 1) begin
            @(negedge clk);
            while (!input_ready) @(negedge clk);
            input_data = expected[i];
            input_valid = 1;
            @(negedge clk);
            input_valid = 0;
        end
        while (consumed != 17 && cycle < 1000) @(posedge clk);
        if (consumed != 17)
            $fatal(1, "FIFO排出タイムアウト: %0d byte", consumed);
        $display("UART FIFOテスト成功");
        $finish;
    end
endmodule
