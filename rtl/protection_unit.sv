// MMU導入前のUser mode向けベース・リミット保護。
// 将来はこのallow結果をMMUおよびcapability checkerの結果と論理積する。
module protection_unit #(
    parameter logic [31:0] KERNEL_MMIO_BASE = 32'h8000_0000
) (
    input  logic        privileged,
    input  logic [31:0] address,
    input  logic [2:0]  access_bytes,
    input  logic [31:0] user_base,
    input  logic [31:0] user_limit,
    output logic        allowed
);
    logic [32:0] end_address;

    always_comb begin
        end_address = {1'b0, address} + {30'b0, access_bytes};
        allowed = privileged ||
            (address < KERNEL_MMIO_BASE && address >= user_base &&
             !end_address[32] && end_address[31:0] <= user_limit);
    end
endmodule
