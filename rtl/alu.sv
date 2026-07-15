module alu (
    input  logic [2:0]  operation,
    input  logic [31:0] lhs,
    input  logic [31:0] rhs,
    output logic [31:0] result
);
    localparam logic [2:0] ALU_ADD = 3'd0, ALU_SUB = 3'd1,
        ALU_AND = 3'd2, ALU_OR = 3'd3, ALU_XOR = 3'd4,
        ALU_SHL = 3'd5, ALU_SHR = 3'd6, ALU_SAR = 3'd7;

    always_comb begin
        case (operation)
            ALU_ADD: result = lhs + rhs;
            ALU_SUB: result = lhs - rhs;
            ALU_AND: result = lhs & rhs;
            ALU_OR:  result = lhs | rhs;
            ALU_XOR: result = lhs ^ rhs;
            ALU_SHL: result = lhs << rhs[4:0];
            ALU_SHR: result = lhs >> rhs[4:0];
            ALU_SAR: result = $signed(lhs) >>> rhs[4:0];
            default: result = 32'b0;
        endcase
    end
endmodule
