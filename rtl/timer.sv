module indigo_timer (
    input  logic        clk,
    input  logic        reset,
    input  logic        write_compare_low,
    input  logic        write_compare_high,
    input  logic        write_control,
    input  logic [31:0] write_data,
    output logic [63:0] count,
    output logic [63:0] compare,
    output logic        enabled,
    output logic        pending
);
    assign pending = enabled && (count >= compare);

    always_ff @(posedge clk) begin
        if (reset) begin
            count <= 64'b0;
            compare <= 64'hffff_ffff_ffff_ffff;
            enabled <= 1'b0;
        end else begin
            count <= count + 1'b1;
            if (write_compare_low)
                compare[31:0] <= write_data;
            if (write_compare_high)
                compare[63:32] <= write_data;
            if (write_control)
                enabled <= write_data[0];
        end
    end
endmodule
