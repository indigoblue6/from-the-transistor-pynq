module uart_rx #(
    parameter integer CLOCKS_PER_BIT = 271,
    parameter integer FIFO_DEPTH = 16,
    parameter integer POINTER_WIDTH = $clog2(FIFO_DEPTH)
) (
    input  logic        clk,
    input  logic        reset,
    input  logic        rx,
    input  logic        pop,
    input  logic        control_write,
    input  logic [31:0] control_data,
    output logic [7:0]  data,
    output logic        available,
    output logic        overrun,
    output logic        interrupt_enable,
    output logic        interrupt_request
);
    typedef enum logic [1:0] {RX_IDLE, RX_START, RX_DATA, RX_STOP} rx_state_t;
    rx_state_t rx_state;
    logic rx_meta, rx_sync;
    integer sample_counter;
    logic [2:0] bit_index;
    logic [7:0] shift_register;
    logic received_valid;
    logic [7:0] received_data;

    logic [7:0] fifo [0:FIFO_DEPTH-1];
    logic [POINTER_WIDTH-1:0] read_pointer, write_pointer;
    logic [POINTER_WIDTH:0] fifo_count;

    assign data = fifo[read_pointer];
    assign available = fifo_count != 0;
    assign interrupt_request = interrupt_enable && available;

    // 非同期RX端子は2段フリップフロップでCPUクロックへ同期する。
    always_ff @(posedge clk) begin
        if (reset) begin
            rx_meta <= 1'b1;
            rx_sync <= 1'b1;
        end else begin
            rx_meta <= rx;
            rx_sync <= rx_meta;
        end
    end

    // start bit中央を確認し、各data bit中央とstop bitを順に標本化する。
    always_ff @(posedge clk) begin
        if (reset) begin
            rx_state <= RX_IDLE;
            sample_counter <= 0;
            bit_index <= 3'b0;
            shift_register <= 8'b0;
            received_valid <= 1'b0;
            received_data <= 8'b0;
        end else begin
            received_valid <= 1'b0;
            case (rx_state)
                RX_IDLE: begin
                    if (!rx_sync) begin
                        sample_counter <= CLOCKS_PER_BIT / 2;
                        rx_state <= RX_START;
                    end
                end
                RX_START: begin
                    if (sample_counter == 0) begin
                        if (!rx_sync) begin
                            sample_counter <= CLOCKS_PER_BIT - 1;
                            bit_index <= 3'b0;
                            rx_state <= RX_DATA;
                        end else begin
                            rx_state <= RX_IDLE;
                        end
                    end else begin
                        sample_counter <= sample_counter - 1;
                    end
                end
                RX_DATA: begin
                    if (sample_counter == 0) begin
                        shift_register[bit_index] <= rx_sync;
                        sample_counter <= CLOCKS_PER_BIT - 1;
                        if (bit_index == 3'd7)
                            rx_state <= RX_STOP;
                        else
                            bit_index <= bit_index + 1'b1;
                    end else begin
                        sample_counter <= sample_counter - 1;
                    end
                end
                RX_STOP: begin
                    if (sample_counter == 0) begin
                        if (rx_sync) begin
                            received_data <= shift_register;
                            received_valid <= 1'b1;
                        end
                        rx_state <= RX_IDLE;
                    end else begin
                        sample_counter <= sample_counter - 1;
                    end
                end
                default: rx_state <= RX_IDLE;
            endcase
        end
    end

    always_ff @(posedge clk) begin
        if (reset) begin
            read_pointer <= '0;
            write_pointer <= '0;
            fifo_count <= '0;
            overrun <= 1'b0;
            interrupt_enable <= 1'b0;
        end else begin
            if (control_write) begin
                interrupt_enable <= control_data[0];
                if (control_data[1])
                    overrun <= 1'b0;
            end
            case ({received_valid, pop && available})
                2'b10: begin
                    if (fifo_count == (POINTER_WIDTH + 1)'(FIFO_DEPTH)) begin
                        overrun <= 1'b1;
                    end else begin
                        fifo[write_pointer] <= received_data;
                        write_pointer <= write_pointer + 1'b1;
                        fifo_count <= fifo_count + 1'b1;
                    end
                end
                2'b01: begin
                    read_pointer <= read_pointer + 1'b1;
                    fifo_count <= fifo_count - 1'b1;
                end
                2'b11: begin
                    fifo[write_pointer] <= received_data;
                    write_pointer <= write_pointer + 1'b1;
                    read_pointer <= read_pointer + 1'b1;
                end
                default: begin end
            endcase
        end
    end
endmodule
