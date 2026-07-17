module decoder (
    input  logic [31:0] instruction,
    output logic [5:0]  opcode,
    output logic [3:0]  reg_a,
    output logic [3:0]  reg_b,
    output logic [3:0]  reg_c,
    output logic [31:0] immediate18,
    output logic [31:0] immediate22,
    output logic [31:0] offset26,
    output logic        illegal,
    output logic        reserved_violation
);
    localparam logic [5:0] OP_NOP = 6'h00, OP_ADD = 6'h01, OP_SAR = 6'h08,
        OP_MOVI = 6'h09, OP_ADDI = 6'h0a, OP_LUI = 6'h0b,
        OP_LOAD = 6'h0c, OP_STOREB = 6'h0f, OP_BEQ = 6'h10,
        OP_BGE = 6'h13, OP_JMP = 6'h14, OP_CALL = 6'h15,
        OP_RET = 6'h16, OP_HALT = 6'h17, OP_CSRR = 6'h18,
        OP_CSRW = 6'h19, OP_ERET = 6'h1a, OP_ECALL = 6'h1b, OP_WFI = 6'h1c,
        OP_CSRSET = 6'h1d, OP_CSRCLR = 6'h1e;

    always_comb begin
        opcode = instruction[31:26];
        reg_a = instruction[25:22];
        reg_b = instruction[21:18];
        reg_c = instruction[17:14];
        immediate18 = {{14{instruction[17]}}, instruction[17:0]};
        immediate22 = {{10{instruction[21]}}, instruction[21:0]};
        offset26 = {{6{instruction[25]}}, instruction[25:0]};
        illegal = 1'b0;
        reserved_violation = 1'b0;
        case (instruction[31:26])
            OP_NOP, OP_RET, OP_HALT, OP_ERET, OP_ECALL, OP_WFI:
                illegal = (instruction[25:0] != 26'b0);
            OP_ADD, 6'h02, 6'h03, 6'h04, 6'h05, 6'h06, 6'h07, OP_SAR:
                illegal = (instruction[13:0] != 14'b0);
            OP_MOVI, OP_ADDI, OP_LOAD, 6'h0d, 6'h0e, OP_STOREB,
            OP_BEQ, 6'h11, 6'h12, OP_BGE, OP_JMP, OP_CALL:
                illegal = 1'b0;
            OP_LUI:
                illegal = (instruction[21:16] != 6'b0);
            OP_CSRR, OP_CSRW, OP_CSRSET, OP_CSRCLR:
                illegal = (instruction[21:8] != 14'b0);
            default:
                illegal = 1'b1;
        endcase
        reserved_violation = illegal && instruction[31:26] <= OP_CSRCLR;
    end
endmodule
