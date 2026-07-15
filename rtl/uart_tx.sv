module uart_tx #(
    parameter integer CLOCKS_PER_BIT = 868
) (
    input  logic       clk,
    input  logic       reset,
    input  logic       valid,
    input  logic [7:0] data,
    output logic       ready,
    output logic       tx
);
    logic [9:0] shift_register;
    logic [3:0] bit_count;
    integer clock_count;

    assign ready = (bit_count == 0);
    assign tx = (bit_count == 0) ? 1'b1 : shift_register[0];

    always_ff @(posedge clk) begin
        if (reset) begin
            shift_register <= 10'h3ff;
            bit_count <= 0;
            clock_count <= 0;
        end else if (bit_count == 0) begin
            if (valid) begin
                // 送信順はstart、データLSBからMSB、stop。
                shift_register <= {1'b1, data, 1'b0};
                bit_count <= 10;
                clock_count <= CLOCKS_PER_BIT - 1;
            end
        end else if (clock_count == 0) begin
            shift_register <= {1'b1, shift_register[9:1]};
            bit_count <= bit_count - 1'b1;
            clock_count <= CLOCKS_PER_BIT - 1;
        end else begin
            clock_count <= clock_count - 1;
        end
    end
endmodule
