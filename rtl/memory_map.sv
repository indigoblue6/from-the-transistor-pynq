module memory_map #(
    parameter string INSTRUCTION_INIT_FILE = ""
) (
    input logic clk, input logic reset,
    input logic [31:0] instruction_address, output logic [31:0] instruction_data,
    input logic data_valid, input logic data_write, input logic data_byte,
    input logic [31:0] data_address, input logic [31:0] data_write_data,
    output logic [31:0] data_read_data, output logic data_fault,
    output logic uart_valid, output logic [7:0] uart_data,
    output logic sim_exit_valid, output logic [31:0] sim_exit_code
);
    localparam logic [31:0] RAM_BASE = 32'h0000_4000, RAM_END = 32'h0001_0000;
    localparam logic [31:0] UART_TX = 32'h8000_0000, UART_STATUS = 32'h8000_0004;
    localparam logic [31:0] SIM_EXIT = 32'h8000_1000;
    (* ram_style = "block" *) logic [31:0] instruction_memory [0:4095];
    (* ram_style = "block" *) logic [7:0] lane0 [0:12287];
    (* ram_style = "block" *) logic [7:0] lane1 [0:12287];
    (* ram_style = "block" *) logic [7:0] lane2 [0:12287];
    (* ram_style = "block" *) logic [7:0] lane3 [0:12287];
    logic [7:0] lane_q0, lane_q1, lane_q2, lane_q3;
    logic [13:0] word_index;
    logic ram_access, ram_write;
    logic write0, write1, write2, write3;

    assign word_index = data_address[15:2] - 14'h1000;
    assign ram_access = data_valid && data_address >= RAM_BASE && data_address < RAM_END &&
                        (data_byte || data_address[1:0] == 0);
    assign ram_write = ram_access && data_write;
    assign write0 = ram_write && (!data_byte || data_address[1:0] == 0);
    assign write1 = ram_write && (!data_byte || data_address[1:0] == 1);
    assign write2 = ram_write && (!data_byte || data_address[1:0] == 2);
    assign write3 = ram_write && (!data_byte || data_address[1:0] == 3);

    always_comb begin
        if (data_address == UART_STATUS)
            data_read_data = 32'd1;
        else if (!data_byte)
            data_read_data = {lane_q3, lane_q2, lane_q1, lane_q0};
        else case (data_address[1:0])
            0: data_read_data = {24'b0, lane_q0};
            1: data_read_data = {24'b0, lane_q1};
            2: data_read_data = {24'b0, lane_q2};
            default: data_read_data = {24'b0, lane_q3};
        endcase
    end

    initial if (INSTRUCTION_INIT_FILE != "")
        $readmemh(INSTRUCTION_INIT_FILE, instruction_memory);

    // 各laneは1読出し・1書込みの単純な同期Block RAMテンプレートである。
    always_ff @(posedge clk) begin
        instruction_data <= instruction_memory[instruction_address[13:2]];
        if (ram_access) begin
            lane_q0 <= lane0[word_index]; lane_q1 <= lane1[word_index];
            lane_q2 <= lane2[word_index]; lane_q3 <= lane3[word_index];
            if (write0) lane0[word_index] <= data_write_data[7:0];
            if (write1) lane1[word_index] <= data_byte ? data_write_data[7:0] : data_write_data[15:8];
            if (write2) lane2[word_index] <= data_byte ? data_write_data[7:0] : data_write_data[23:16];
            if (write3) lane3[word_index] <= data_byte ? data_write_data[7:0] : data_write_data[31:24];
        end
    end

    always_ff @(posedge clk) begin
        uart_valid <= 1'b0; sim_exit_valid <= 1'b0; data_fault <= 1'b0;
        if (reset) begin
            uart_data <= 0; sim_exit_code <= 0;
        end else if (data_valid) begin
            if (data_write && data_address == UART_TX) begin
                uart_data <= data_write_data[7:0]; uart_valid <= 1'b1;
            end else if (data_write && data_address == SIM_EXIT) begin
                sim_exit_code <= data_write_data; sim_exit_valid <= 1'b1;
            end else if (!ram_access && !(data_address == UART_STATUS && !data_write)) begin
                data_fault <= 1'b1;
            end
        end
    end
endmodule
