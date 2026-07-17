module csr_file (
    input  logic        clk,
    input  logic        reset,
    input  logic [7:0]  read_number,
    output logic [31:0] read_data,
    output logic        read_valid,
    input  logic        write_enable,
    input  logic [7:0]  write_number,
    input  logic [31:0] write_data,
    output logic        write_valid,
    input  logic        trap_enter,
    input  logic [31:0] trap_epc,
    input  logic [31:0] trap_cause,
    input  logic [31:0] trap_badaddr,
    input  logic        eret,
    input  logic        uart_rx_pending,
    input  logic        external_irq,
    output logic [31:0] status_value,
    output logic [31:0] epc_value,
    output logic [31:0] tvec_value,
    output logic [31:0] user_base_value,
    output logic [31:0] user_limit_value,
    output logic [2:0]  interrupt_pending,
    output logic [2:0]  interrupt_enable,
    output logic [63:0] timer_count,
    output logic [31:0] cause_value,
    output logic [31:0] badaddr_value,
    output logic [31:0] kernel_sp_value,
    output logic        trap_active
);
    localparam logic [7:0] CSR_STATUS = 8'h00, CSR_EPC = 8'h01,
        CSR_CAUSE = 8'h02, CSR_TVEC = 8'h03, CSR_BADADDR = 8'h04,
        CSR_TIMER_COUNT_LO = 8'h05, CSR_TIMER_COUNT_HI = 8'h06,
        CSR_TIMER_COMPARE_LO = 8'h07, CSR_TIMER_COMPARE_HI = 8'h08,
        CSR_INTERRUPT_PENDING = 8'h09, CSR_INTERRUPT_ENABLE = 8'h0a,
        CSR_USER_BASE = 8'h0b, CSR_USER_LIMIT = 8'h0c,
        CSR_KERNEL_SP = 8'h0d, CSR_SCRATCH = 8'h0e,
        CSR_TIMER_CONTROL = 8'h0f;

    logic [31:0] scratch_value;
    logic software_pending, external_pending;
    logic [63:0] timer_compare;
    logic timer_enabled, timer_pending;
    logic write_compare_low, write_compare_high, write_timer_control;

    assign write_compare_low = write_enable && write_valid && write_number == CSR_TIMER_COMPARE_LO;
    assign write_compare_high = write_enable && write_valid && write_number == CSR_TIMER_COMPARE_HI;
    assign write_timer_control = write_enable && write_valid && write_number == CSR_TIMER_CONTROL;
    assign interrupt_pending = {software_pending, external_pending | uart_rx_pending, timer_pending};

    indigo_timer timer_i (
        .clk(clk), .reset(reset),
        .write_compare_low(write_compare_low),
        .write_compare_high(write_compare_high),
        .write_control(write_timer_control), .write_data(write_data),
        .count(timer_count), .compare(timer_compare),
        .enabled(timer_enabled), .pending(timer_pending)
    );

    always_comb begin
        read_data = 32'b0;
        read_valid = 1'b1;
        case (read_number)
            CSR_STATUS: read_data = status_value;
            CSR_EPC: read_data = epc_value;
            CSR_CAUSE: read_data = cause_value;
            CSR_TVEC: read_data = tvec_value;
            CSR_BADADDR: read_data = badaddr_value;
            CSR_TIMER_COUNT_LO: read_data = timer_count[31:0];
            CSR_TIMER_COUNT_HI: read_data = timer_count[63:32];
            CSR_TIMER_COMPARE_LO: read_data = timer_compare[31:0];
            CSR_TIMER_COMPARE_HI: read_data = timer_compare[63:32];
            CSR_INTERRUPT_PENDING: read_data = {29'b0, interrupt_pending};
            CSR_INTERRUPT_ENABLE: read_data = {29'b0, interrupt_enable};
            CSR_USER_BASE: read_data = user_base_value;
            CSR_USER_LIMIT: read_data = user_limit_value;
            CSR_KERNEL_SP: read_data = kernel_sp_value;
            CSR_SCRATCH: read_data = scratch_value;
            CSR_TIMER_CONTROL: read_data = {31'b0, timer_enabled};
            default: read_valid = 1'b0;
        endcase
    end

    always_comb begin
        write_valid = 1'b1;
        case (write_number)
            CSR_STATUS, CSR_EPC, CSR_TIMER_COMPARE_LO, CSR_TIMER_COMPARE_HI,
            CSR_INTERRUPT_PENDING, CSR_INTERRUPT_ENABLE, CSR_USER_BASE,
            CSR_USER_LIMIT, CSR_KERNEL_SP, CSR_SCRATCH, CSR_TIMER_CONTROL:
                write_valid = 1'b1;
            CSR_TVEC: write_valid = (write_data[1:0] == 2'b00);
            default: write_valid = 1'b0;
        endcase
    end

    always_ff @(posedge clk) begin
        if (reset) begin
            status_value <= 32'h0000_0004;
            epc_value <= 32'b0;
            cause_value <= 32'b0;
            tvec_value <= 32'b0;
            badaddr_value <= 32'b0;
            interrupt_enable <= 3'b0;
            software_pending <= 1'b0;
            external_pending <= 1'b0;
            trap_active <= 1'b0;
            user_base_value <= 32'b0;
            user_limit_value <= 32'b0;
            kernel_sp_value <= 32'h0001_0000;
            scratch_value <= 32'b0;
        end else begin
            if (external_irq)
                external_pending <= 1'b1;
            else if (write_enable && write_valid &&
                     write_number == CSR_INTERRUPT_PENDING && write_data[1])
                external_pending <= 1'b0;
            if (trap_enter) begin
                trap_active <= 1'b1;
                status_value[1] <= status_value[0];
                status_value[3] <= status_value[2];
                status_value[0] <= 1'b0;
                status_value[2] <= 1'b1;
                epc_value <= trap_epc;
                cause_value <= trap_cause;
                badaddr_value <= trap_badaddr;
            end else if (eret) begin
                trap_active <= 1'b0;
                status_value[0] <= status_value[1];
                status_value[2] <= status_value[3];
                status_value[1] <= 1'b0;
                status_value[3] <= 1'b1;
            end else if (write_enable && write_valid) begin
                case (write_number)
                    CSR_STATUS: status_value <= write_data & 32'h0000_000f;
                    CSR_EPC: epc_value <= write_data;
                    CSR_TVEC: tvec_value <= write_data;
                    CSR_INTERRUPT_PENDING: software_pending <= write_data[2];
                    CSR_INTERRUPT_ENABLE: interrupt_enable <= write_data[2:0];
                    CSR_USER_BASE: user_base_value <= write_data;
                    CSR_USER_LIMIT: user_limit_value <= write_data;
                    CSR_KERNEL_SP: kernel_sp_value <= write_data;
                    CSR_SCRATCH: scratch_value <= write_data;
                    default: begin end
                endcase
            end
        end
    end
endmodule
