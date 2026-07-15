module uart_fifo (
    input  logic       clk,
    input  logic       reset,
    input  logic       input_valid,
    input  logic [7:0] input_data,
    output logic       input_ready,
    output logic       output_valid,
    output logic [7:0] output_data,
    input  logic       output_ready
);
    logic [7:0] memory [0:31];
    logic [4:0] write_pointer, read_pointer;
    logic [5:0] count;
    localparam logic [5:0] DEPTH_COUNT = 6'd32;
    logic push, pop;

    assign input_ready = (count < DEPTH_COUNT);
    assign output_valid = (count != 0);
    assign output_data = memory[read_pointer];
    assign push = input_valid && input_ready;
    assign pop = output_valid && output_ready;

    // MMIOの連続文字書込みを物理UARTの送信速度へ合わせる。
    always_ff @(posedge clk) begin
        if (reset) begin
            write_pointer <= 0;
            read_pointer <= 0;
            count <= 0;
        end else begin
            if (push) begin
                memory[write_pointer] <= input_data;
                write_pointer <= write_pointer + 1'b1;
            end
            if (pop)
                read_pointer <= read_pointer + 1'b1;
            case ({push, pop})
                2'b10: count <= count + 1'b1;
                2'b01: count <= count - 1'b1;
                default: count <= count;
            endcase
        end
    end
endmodule
