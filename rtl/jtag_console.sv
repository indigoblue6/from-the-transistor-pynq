// USB-JTAGのVIOとIndigo32のUARTバイトストリームを接続するデバッグコンソール。
// host_in[9:8]はトグル方式であり、VIOの更新周期に依存しない。
module jtag_console #(
    parameter integer FIFO_DEPTH = 16
) (
    input  logic       clk,
    input  logic       reset,
    input  logic [9:0] host_in,
    output logic [8:0] host_out,
    input  logic       tx_push,
    input  logic [7:0] tx_data,
    output logic       rx_available,
    output logic [7:0] rx_data,
    input  logic       rx_pop
);
    logic [7:0] rx_mem [0:FIFO_DEPTH-1];
    logic [7:0] tx_mem [0:FIFO_DEPTH-1];
    integer rx_wr, rx_rd, rx_count;
    integer tx_wr, tx_rd, tx_count;
    logic rx_toggle_seen, tx_ack_seen;
    logic [7:0] tx_head;

    assign rx_available = (rx_count != 0);
    assign rx_data = (rx_count != 0) ? rx_mem[rx_rd] : 8'h00;
    assign tx_head = (tx_count != 0) ? tx_mem[tx_rd] : 8'h00;
    assign host_out = { (tx_count != 0), tx_head };

    always_ff @(posedge clk) begin
        if (reset) begin
            rx_wr <= 0; rx_rd <= 0; rx_count <= 0;
            tx_wr <= 0; tx_rd <= 0; tx_count <= 0;
            rx_toggle_seen <= 1'b0;
            tx_ack_seen <= 1'b0;
        end else begin
            // VIOからのRX文字を一度だけFIFOへ取り込む。
            if (host_in[9] != rx_toggle_seen) begin
                rx_toggle_seen <= host_in[9];
                if (rx_count < FIFO_DEPTH) begin
                    rx_mem[rx_wr] <= host_in[7:0];
                    rx_wr <= (rx_wr + 1) % FIFO_DEPTH;
                    rx_count <= rx_count + 1;
                end
            end
            if (rx_pop && rx_count != 0) begin
                rx_rd <= (rx_rd + 1) % FIFO_DEPTH;
                rx_count <= rx_count - 1;
            end

            // UART TX文字を保持し、ホストのackトグルで消費する。
            if (tx_push && tx_count < FIFO_DEPTH) begin
                tx_mem[tx_wr] <= tx_data;
                tx_wr <= (tx_wr + 1) % FIFO_DEPTH;
                tx_count <= tx_count + 1;
            end
            if (host_in[8] != tx_ack_seen && tx_count != 0) begin
                tx_ack_seen <= host_in[8];
                tx_rd <= (tx_rd + 1) % FIFO_DEPTH;
                tx_count <= tx_count - 1;
            end else if (host_in[8] != tx_ack_seen) begin
                tx_ack_seen <= host_in[8];
            end
        end
    end
endmodule
