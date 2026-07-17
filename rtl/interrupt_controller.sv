module interrupt_controller (
    input  logic        global_enable,
    input  logic [2:0]  pending,
    input  logic [2:0]  enable,
    output logic        request,
    output logic [31:0] cause
);
    logic [2:0] active;

    always_comb begin
        active = pending & enable;
        request = global_enable && (active != 3'b000);
        if (active[0])
            cause = 32'h8000_0008;
        else if (active[1])
            cause = 32'h8000_0009;
        else if (active[2])
            cause = 32'h8000_000a;
        else
            cause = 32'b0;
    end
endmodule
