module register_file (
    input  logic        clk,
    input  logic        reset,
    input  logic [3:0]  read_index1,
    input  logic [3:0]  read_index2,
    output logic [31:0] read_data1,
    output logic [31:0] read_data2,
    input  logic [3:0]  read_index3,
    output logic [31:0] read_data3,
    input  logic        write_enable,
    input  logic [3:0]  write_index,
    input  logic [31:0] write_data
);
    logic [31:0] registers [0:15];
    integer i;

    assign read_data1 = (read_index1 == 4'd0) ? 32'b0 : registers[read_index1];
    assign read_data2 = (read_index2 == 4'd0) ? 32'b0 : registers[read_index2];
    assign read_data3 = (read_index3 == 4'd0) ? 32'b0 : registers[read_index3];

    // 同期アクティブHighリセットを全RTLで統一する。
    always_ff @(posedge clk) begin
        if (reset) begin
            for (i = 0; i < 16; i = i + 1)
                registers[i] <= 32'b0;
            registers[15] <= 32'h0001_0000;
        end else if (write_enable && write_index != 4'd0) begin
            registers[write_index] <= write_data;
        end
    end
endmodule
