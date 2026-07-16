module cpu (
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
    input  logic        uart_rx_pending,
    output logic        halted,
    output logic        faulted,
    output logic [31:0] debug_pc,
    output logic [31:0] debug_instruction,
    output logic [3:0]  debug_state,
    output logic        debug_register_write,
    output logic [3:0]  debug_register_index,
    output logic [31:0] debug_register_data,
    output logic        debug_privileged,
    output logic [2:0]  debug_interrupt_pending,
    output logic [2:0]  debug_interrupt_enable,
    output logic [31:0] debug_epc,
    output logic [31:0] debug_cause,
    output logic [31:0] debug_badaddr,
    output logic [31:0] debug_timer_count,
    output logic        debug_interrupt_taken
);
    localparam logic [5:0] OP_NOP = 6'h00, OP_ADD = 6'h01, OP_SUB = 6'h02,
        OP_AND = 6'h03, OP_OR = 6'h04, OP_XOR = 6'h05, OP_SHL = 6'h06,
        OP_SHR = 6'h07, OP_SAR = 6'h08, OP_MOVI = 6'h09, OP_ADDI = 6'h0a,
        OP_LUI = 6'h0b, OP_LOAD = 6'h0c, OP_STORE = 6'h0d,
        OP_LOADB = 6'h0e, OP_STOREB = 6'h0f, OP_BEQ = 6'h10,
        OP_BNE = 6'h11, OP_BLT = 6'h12, OP_BGE = 6'h13,
        OP_JMP = 6'h14, OP_CALL = 6'h15, OP_RET = 6'h16, OP_HALT = 6'h17,
        OP_CSRR = 6'h18, OP_CSRW = 6'h19, OP_ERET = 6'h1a,
        OP_ECALL = 6'h1b, OP_WFI = 6'h1c;
    localparam logic [7:0] CSR_TIMER_COUNT_LO = 8'h05,
        CSR_TIMER_COUNT_HI = 8'h06;
    localparam logic [31:0] CAUSE_ILLEGAL = 32'd0,
        CAUSE_FETCH_MISALIGNED = 32'd1, CAUSE_FETCH_ACCESS = 32'd2,
        CAUSE_LOAD_MISALIGNED = 32'd3, CAUSE_LOAD_ACCESS = 32'd4,
        CAUSE_STORE_MISALIGNED = 32'd5, CAUSE_STORE_ACCESS = 32'd6,
        CAUSE_ECALL = 32'd7, CAUSE_PRIVILEGED = 32'd11;

    typedef enum logic [3:0] {
        ST_FETCH, ST_FETCH_WAIT, ST_DECODE, ST_EXECUTE,
        ST_MEMORY_REQUEST, ST_MEMORY_WAIT, ST_WFI, ST_TRAP_WAIT, ST_HALTED, ST_FAULT
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
    logic [31:0] memory_instruction_pc;
    logic [3:0] memory_destination;
    logic memory_is_write, memory_is_byte;

    logic [7:0] csr_read_number, csr_write_number;
    logic [31:0] csr_read_data;
    logic csr_read_valid, csr_write_enable, csr_write_valid;
    logic trap_enter, csr_eret;
    logic [31:0] trap_epc, trap_cause, trap_badaddr;
    logic [31:0] status_value, csr_epc, csr_tvec, user_base, user_limit;
    logic [2:0] interrupt_pending, interrupt_enable;
    logic [63:0] timer_count;
    logic [31:0] csr_cause, csr_badaddr;
    logic interrupt_request;
    logic [31:0] interrupt_cause;
    logic user_csr_read_allowed;
    logic [31:0] effective_address;
    logic [32:0] effective_end;
    logic effective_user_allowed;

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

    assign csr_read_number = instruction_register[7:0];
    assign csr_write_number = instruction_register[7:0];
    assign csr_write_enable = state == ST_EXECUTE && decoded_opcode == OP_CSRW &&
        status_value[2] && csr_write_valid;
    assign user_csr_read_allowed = csr_read_number == CSR_TIMER_COUNT_LO ||
        csr_read_number == CSR_TIMER_COUNT_HI;

    csr_file csr_file_i (
        .clk(clk), .reset(reset), .read_number(csr_read_number),
        .read_data(csr_read_data), .read_valid(csr_read_valid),
        .write_enable(csr_write_enable), .write_number(csr_write_number),
        .write_data(read_data3), .write_valid(csr_write_valid),
        .trap_enter(trap_enter), .trap_epc(trap_epc), .trap_cause(trap_cause),
        .trap_badaddr(trap_badaddr), .eret(csr_eret),
        .uart_rx_pending(uart_rx_pending), .status_value(status_value),
        .epc_value(csr_epc), .tvec_value(csr_tvec),
        .user_base_value(user_base), .user_limit_value(user_limit),
        .interrupt_pending(interrupt_pending), .interrupt_enable(interrupt_enable),
        .timer_count(timer_count), .cause_value(csr_cause),
        .badaddr_value(csr_badaddr)
    );

    interrupt_controller interrupt_controller_i (
        .global_enable(status_value[0]), .pending(interrupt_pending),
        .enable(interrupt_enable), .request(interrupt_request),
        .cause(interrupt_cause)
    );

    assign effective_address = read_data1 + immediate18;
    assign effective_end = {1'b0, effective_address} +
        ((decoded_opcode == OP_LOAD || decoded_opcode == OP_STORE) ? 33'd4 : 33'd1);
    assign effective_user_allowed = status_value[2] ||
        ((read_data1 + immediate18) < 32'h8000_0000 &&
         (read_data1 + immediate18) >= user_base &&
         !effective_end[32] && effective_end[31:0] <= user_limit);

    assign instruction_address = pc;
    assign debug_pc = pc;
    assign debug_instruction = instruction_register;
    assign debug_state = state;
    assign halted = state == ST_HALTED;
    assign faulted = state == ST_FAULT;
    assign debug_register_write = register_write;
    assign debug_register_index = register_write_index;
    assign debug_register_data = register_write_data;
    assign debug_privileged = status_value[2];
    assign debug_interrupt_pending = interrupt_pending;
    assign debug_interrupt_enable = interrupt_enable;
    assign debug_epc = csr_epc;
    assign debug_cause = csr_cause;
    assign debug_badaddr = csr_badaddr;
    assign debug_timer_count = timer_count[31:0];

    always_comb begin
        register_write = 1'b0;
        register_write_index = 4'b0;
        register_write_data = 32'b0;
        data_valid = state == ST_MEMORY_REQUEST;
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
                OP_CSRR: begin
                    if (csr_read_valid && (status_value[2] || user_csr_read_allowed)) begin
                        register_write = 1'b1;
                        register_write_index = decoded_a;
                        register_write_data = csr_read_data;
                    end
                end
                default: begin end
            endcase
        end else if (state == ST_MEMORY_WAIT && !data_fault && !memory_is_write) begin
            register_write = 1'b1;
            register_write_index = memory_destination;
            register_write_data = data_read_data;
        end
    end

    task automatic enter_trap(
        input logic [31:0] requested_epc,
        input logic [31:0] requested_cause,
        input logic [31:0] requested_badaddr
    );
        begin
            if (csr_tvec == 32'b0) begin
                state <= ST_FAULT;
            end else begin
                trap_enter <= 1'b1;
                trap_epc <= requested_epc;
                trap_cause <= requested_cause;
                trap_badaddr <= requested_badaddr;
                pc <= csr_tvec;
                state <= ST_TRAP_WAIT;
            end
        end
    endtask

    always_ff @(posedge clk) begin
        if (reset) begin
            state <= ST_FETCH;
            pc <= 32'b0;
            instruction_register <= 32'b0;
            memory_address_register <= 32'b0;
            memory_write_register <= 32'b0;
            memory_instruction_pc <= 32'b0;
            memory_destination <= 4'b0;
            memory_is_write <= 1'b0;
            memory_is_byte <= 1'b0;
            trap_enter <= 1'b0;
            trap_epc <= 32'b0;
            trap_cause <= 32'b0;
            trap_badaddr <= 32'b0;
            csr_eret <= 1'b0;
            debug_interrupt_taken <= 1'b0;
        end else begin
            trap_enter <= 1'b0;
            csr_eret <= 1'b0;
            debug_interrupt_taken <= 1'b0;
            case (state)
                ST_FETCH: begin
                    if (interrupt_request) begin
                        debug_interrupt_taken <= 1'b1;
                        enter_trap(pc, interrupt_cause, 32'b0);
                    end else if (pc[1:0] != 2'b00) begin
                        enter_trap(pc, CAUSE_FETCH_MISALIGNED, pc);
                    end else if (pc >= 32'h0000_4000 ||
                                 (!status_value[2] &&
                                  (pc < user_base || pc + 32'd4 > user_limit))) begin
                        enter_trap(pc, CAUSE_FETCH_ACCESS, pc);
                    end else begin
                        state <= ST_FETCH_WAIT;
                    end
                end
                ST_FETCH_WAIT: begin
                    instruction_register <= instruction_data;
                    pc <= pc + 32'd4;
                    state <= ST_DECODE;
                end
                ST_DECODE: begin
                    if (decoded_illegal)
                        enter_trap(pc - 32'd4, CAUSE_ILLEGAL, instruction_register);
                    else
                        state <= ST_EXECUTE;
                end
                ST_EXECUTE: begin
                    case (decoded_opcode)
                        OP_NOP, OP_ADD, OP_SUB, OP_AND, OP_OR, OP_XOR,
                        OP_SHL, OP_SHR, OP_SAR, OP_MOVI, OP_ADDI, OP_LUI:
                            state <= ST_FETCH;
                        OP_LOAD, OP_STORE, OP_LOADB, OP_STOREB: begin
                            if ((decoded_opcode == OP_LOAD || decoded_opcode == OP_STORE) &&
                                effective_address[1:0] != 2'b00) begin
                                enter_trap(
                                    pc - 32'd4,
                                    (decoded_opcode == OP_STORE) ?
                                        CAUSE_STORE_MISALIGNED : CAUSE_LOAD_MISALIGNED,
                                    read_data1 + immediate18
                                );
                            end else if (!effective_user_allowed) begin
                                enter_trap(
                                    pc - 32'd4,
                                    (decoded_opcode == OP_STORE || decoded_opcode == OP_STOREB) ?
                                        CAUSE_STORE_ACCESS : CAUSE_LOAD_ACCESS,
                                    read_data1 + immediate18
                                );
                            end else begin
                                memory_address_register <= read_data1 + immediate18;
                                memory_write_register <= read_data3;
                                memory_instruction_pc <= pc - 32'd4;
                                memory_destination <= decoded_a;
                                memory_is_write <= decoded_opcode == OP_STORE ||
                                    decoded_opcode == OP_STOREB;
                                memory_is_byte <= decoded_opcode == OP_LOADB ||
                                    decoded_opcode == OP_STOREB;
                                state <= ST_MEMORY_REQUEST;
                            end
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
                        OP_CSRR: begin
                            if (!status_value[2] && !user_csr_read_allowed)
                                enter_trap(pc - 32'd4, CAUSE_PRIVILEGED, instruction_register);
                            else if (!csr_read_valid)
                                enter_trap(pc - 32'd4, CAUSE_ILLEGAL, instruction_register);
                            else
                                state <= ST_FETCH;
                        end
                        OP_CSRW: begin
                            if (!status_value[2])
                                enter_trap(pc - 32'd4, CAUSE_PRIVILEGED, instruction_register);
                            else if (!csr_write_valid)
                                enter_trap(pc - 32'd4, CAUSE_ILLEGAL, instruction_register);
                            else
                                state <= ST_FETCH;
                        end
                        OP_ERET: begin
                            if (!status_value[2])
                                enter_trap(pc - 32'd4, CAUSE_PRIVILEGED, instruction_register);
                            else begin
                                csr_eret <= 1'b1;
                                pc <= csr_epc;
                                state <= ST_FETCH;
                            end
                        end
                        OP_ECALL: enter_trap(pc, CAUSE_ECALL, 32'b0);
                        OP_WFI: state <= ST_WFI;
                        OP_HALT: state <= ST_HALTED;
                        default: enter_trap(pc - 32'd4, CAUSE_ILLEGAL, instruction_register);
                    endcase
                end
                ST_MEMORY_REQUEST: state <= ST_MEMORY_WAIT;
                ST_MEMORY_WAIT: begin
                    if (data_fault)
                        enter_trap(
                            memory_instruction_pc,
                            memory_is_write ? CAUSE_STORE_ACCESS : CAUSE_LOAD_ACCESS,
                            memory_address_register
                        );
                    else
                        state <= ST_FETCH;
                end
                ST_WFI: begin
                    if (interrupt_request) begin
                        debug_interrupt_taken <= 1'b1;
                        enter_trap(pc, interrupt_cause, 32'b0);
                    end
                end
                ST_TRAP_WAIT: state <= ST_FETCH;
                ST_HALTED: state <= ST_HALTED;
                default: state <= ST_FAULT;
            endcase
        end
    end
endmodule
