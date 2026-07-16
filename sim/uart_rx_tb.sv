module uart_rx_tb;
    localparam integer CLOCKS_PER_BIT = 8;
    logic clk = 1'b0, reset = 1'b1, rx = 1'b1;
    logic pop = 1'b0, control_write = 1'b0;
    logic [31:0] control_data = 32'b0;
    logic [7:0] data;
    logic available, overrun, interrupt_enable, interrupt_request;
    integer value;

    always #5 clk = ~clk;

    uart_rx #(.CLOCKS_PER_BIT(CLOCKS_PER_BIT), .FIFO_DEPTH(16)) dut (.*);

    task automatic send_byte(input logic [7:0] byte_value);
        integer bit_number;
        begin
            rx = 1'b0;
            repeat (CLOCKS_PER_BIT) @(posedge clk);
            for (bit_number = 0; bit_number < 8; bit_number = bit_number + 1) begin
                rx = byte_value[bit_number];
                repeat (CLOCKS_PER_BIT) @(posedge clk);
            end
            rx = 1'b1;
            repeat (CLOCKS_PER_BIT) @(posedge clk);
            repeat (2) @(posedge clk);
        end
    endtask

    task automatic pop_and_expect(input logic [7:0] expected);
        begin
            if (!available || data != expected)
                $fatal(1, "UART RX FIFO順序不一致: actual=%02x expected=%02x", data, expected);
            pop = 1'b1;
            @(posedge clk);
            pop = 1'b0;
            @(posedge clk);
        end
    endtask

    initial begin
        repeat (4) @(posedge clk);
        reset = 1'b0;
        control_data = 32'd1;
        control_write = 1'b1;
        @(posedge clk);
        control_write = 1'b0;

        send_byte(8'h41);
        send_byte(8'h42);
        if (!available || !interrupt_request || overrun)
            $fatal(1, "UART RX available/IRQ初期状態が不正です");
        pop_and_expect(8'h41);
        pop_and_expect(8'h42);
        if (available || interrupt_request)
            $fatal(1, "FIFO空後もavailableまたはIRQが残っています");

        for (value = 0; value < 17; value = value + 1)
            send_byte(value[7:0]);
        if (!overrun)
            $fatal(1, "17 byte目でoverrunを検出しませんでした");
        for (value = 0; value < 16; value = value + 1)
            pop_and_expect(value[7:0]);

        control_data = 32'd3;
        control_write = 1'b1;
        @(posedge clk);
        control_write = 1'b0;
        @(posedge clk);
        if (overrun)
            $fatal(1, "overrun clearが機能しません");
        $display("UART RX 8N1/FIFO/IRQテスト成功");
        $finish;
    end
endmodule
