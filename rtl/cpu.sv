module cpu_legacy (
    input  logic        clk,
    input  logic        reset,
    output logic [31:0] instruction_address,
    input  logic [31:0] instruction_data,
    output logic        data_valid,
    output logic        data_write,
    output logic        data_byte,
    output logic [31:0] data_address,
    output logic [31:0] data_write_data,
    input  logic [31:0] data_read_data,
    input  logic        data_fault,
    output logic        halted,
    output logic        faulted,
    output logic [31:0] debug_pc,
    output logic [31:0] debug_instruction,
    output logic [3:0]  debug_state,
    output logic        debug_register_write,
    output logic [3:0]  debug_register_index,
    output logic [31:0] debug_register_data
);
    localparam logic [5:0] OP_NOP = 6'h00, OP_ADD = 6'h01, OP_SUB = 6'h02,
        OP_AND = 6'h03, OP_OR = 6'h04, OP_XOR = 6'h05, OP_SHL = 6'h06,
        OP_SHR = 6'h07, OP_SAR = 6'h08, OP_MOVI = 6'h09, OP_ADDI = 6'h0a,
        OP_LUI = 6'h0b, OP_LOAD = 6'h0c, OP_STORE = 6'h0d,
        OP_LOADB = 6'h0e, OP_STOREB = 6'h0f, OP_BEQ = 6'h10,
        OP_BNE = 6'h11, OP_BLT = 6'h12, OP_BGE = 6'h13,
        OP_JMP = 6'h14, OP_CALL = 6'h15, OP_RET = 6'h16, OP_HALT = 6'h17;

    typedef enum logic [3:0] {
        ST_FETCH, ST_FETCH_WAIT, ST_DECODE, ST_EXECUTE,
        ST_MEMORY_REQUEST, ST_MEMORY_WAIT, ST_HALTED, ST_FAULT
    } state_t;
    state_t state;

    logic [31:0] pc, instruction_register;
    logic [5:0] decoded_opcode;
    logic [3:0] decoded_a, decoded_b, decoded_c;
    logic [31:0] immediate18, immediate22, offset26;
    logic decoded_illegal;
    logic [3:0] read_index1, read_index2;
    logic [31:0] read_data1, read_data2, read_data3;
    logic register_write;
    logic [3:0] register_write_index;
    logic [31:0] register_write_data;
    logic [2:0] alu_operation;
    logic [31:0] alu_result;
    logic [31:0] memory_address_register, memory_write_register;
    logic [3:0] memory_destination;
    logic memory_is_write, memory_is_byte;

    decoder decoder_i (
        .instruction(instruction_register), .opcode(decoded_opcode),
        .reg_a(decoded_a), .reg_b(decoded_b), .reg_c(decoded_c),
        .immediate18(immediate18), .immediate22(immediate22),
        .offset26(offset26), .illegal(decoded_illegal)
    );

    assign read_index1 = (decoded_opcode == OP_RET) ? 4'd14 : decoded_b;
    assign read_index2 = decoded_c;
    register_file register_file_i (
        .clk(clk), .reset(reset), .read_index1(read_index1), .read_index2(read_index2),
        .read_data1(read_data1), .read_data2(read_data2),
        .read_index3(decoded_a), .read_data3(read_data3),
        .write_enable(register_write), .write_index(register_write_index),
        .write_data(register_write_data)
    );

    assign alu_operation = decoded_opcode[2:0] - 3'd1;
    alu alu_i (.operation(alu_operation), .lhs(read_data1), .rhs(read_data2), .result(alu_result));

    assign instruction_address = pc;
    assign debug_pc = pc;
    assign debug_instruction = instruction_register;
    assign debug_state = state;
    assign halted = (state == ST_HALTED);
    assign faulted = (state == ST_FAULT);
    assign debug_register_write = register_write;
    assign debug_register_index = register_write_index;
    assign debug_register_data = register_write_data;

    always_comb begin
        register_write = 1'b0;
        register_write_index = 4'b0;
        register_write_data = 32'b0;
        data_valid = 1'b0;
        data_write = memory_is_write;
        data_byte = memory_is_byte;
        data_address = memory_address_register;
        data_write_data = memory_write_register;

        if (state == ST_EXECUTE) begin
            case (decoded_opcode)
                OP_ADD, OP_SUB, OP_AND, OP_OR, OP_XOR, OP_SHL, OP_SHR, OP_SAR: begin
                    register_write = 1'b1;
                    register_write_index = decoded_a;
                    register_write_data = alu_result;
                end
                OP_MOVI: begin
                    register_write = 1'b1;
                    register_write_index = decoded_a;
                    register_write_data = immediate22;
                end
                OP_ADDI: begin
                    register_write = 1'b1;
                    register_write_index = decoded_a;
                    register_write_data = read_data1 + immediate18;
                end
                OP_LUI: begin
                    register_write = 1'b1;
                    register_write_index = decoded_a;
                    register_write_data = {instruction_register[15:0], 16'b0};
                end
                OP_CALL: begin
                    register_write = 1'b1;
                    register_write_index = 4'd14;
                    register_write_data = pc;
                end
                default: begin end
            endcase
        end else if (state == ST_MEMORY_WAIT && !data_fault && !memory_is_write) begin
            register_write = 1'b1;
            register_write_index = memory_destination;
            register_write_data = data_read_data;
        end
        if (state == ST_MEMORY_REQUEST)
            data_valid = 1'b1;
    end

    always_ff @(posedge clk) begin
        if (reset) begin
            state <= ST_FETCH;
            pc <= 32'b0;
            instruction_register <= 32'b0;
            memory_address_register <= 32'b0;
            memory_write_register <= 32'b0;
            memory_destination <= 4'b0;
            memory_is_write <= 1'b0;
            memory_is_byte <= 1'b0;
        end else begin
            case (state)
                ST_FETCH: begin
                    if (pc[1:0] != 2'b00 || pc >= 32'h0000_4000)
                        state <= ST_FAULT;
                    else
                        state <= ST_FETCH_WAIT;
                end
                ST_FETCH_WAIT: begin
                    instruction_register <= instruction_data;
                    pc <= pc + 32'd4;
                    state <= ST_DECODE;
                end
                ST_DECODE: begin
                    state <= decoded_illegal ? ST_FAULT : ST_EXECUTE;
                end
                ST_EXECUTE: begin
                    case (decoded_opcode)
                        OP_NOP, OP_ADD, OP_SUB, OP_AND, OP_OR, OP_XOR,
                        OP_SHL, OP_SHR, OP_SAR, OP_MOVI, OP_ADDI, OP_LUI:
                            state <= ST_FETCH;
                        OP_LOAD, OP_STORE, OP_LOADB, OP_STOREB: begin
                            memory_address_register <= read_data1 + immediate18;
                            memory_write_register <= read_data3;
                            memory_destination <= decoded_a;
                            memory_is_write <= (decoded_opcode == OP_STORE || decoded_opcode == OP_STOREB);
                            memory_is_byte <= (decoded_opcode == OP_LOADB || decoded_opcode == OP_STOREB);
                            state <= ST_MEMORY_REQUEST;
                        end
                        OP_BEQ, OP_BNE, OP_BLT, OP_BGE: begin
                            if ((decoded_opcode == OP_BEQ && read_data3 == read_data1) ||
                                (decoded_opcode == OP_BNE && read_data3 != read_data1) ||
                                (decoded_opcode == OP_BLT && $signed(read_data3) < $signed(read_data1)) ||
                                (decoded_opcode == OP_BGE && $signed(read_data3) >= $signed(read_data1)))
                                pc <= pc + (immediate18 << 2);
                            state <= ST_FETCH;
                        end
                        OP_JMP, OP_CALL: begin
                            pc <= pc + (offset26 << 2);
                            state <= ST_FETCH;
                        end
                        OP_RET: begin
                            pc <= read_data1;
                            state <= ST_FETCH;
                        end
                        OP_HALT: state <= ST_HALTED;
                        default: state <= ST_FAULT;
                    endcase
                end
                ST_MEMORY_REQUEST: state <= ST_MEMORY_WAIT;
                ST_MEMORY_WAIT: state <= data_fault ? ST_FAULT : ST_FETCH;
                ST_HALTED: state <= ST_HALTED;
                default: state <= ST_FAULT;
            endcase
        end
    end
endmodule
