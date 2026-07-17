set_param general.maxThreads 2
# Vivado 2025.2でPYNQ-Z1用bitstreamとILAプローブ定義を生成する。
set root [file normalize [file join [file dirname [info script]] ..]]
set build [file join $root build hardware]
file mkdir $build
create_project -force pynq_cpu $build -part xc7z020clg400-1
set_property target_language Verilog [current_project]
add_files [glob [file join $root rtl *.sv]]
add_files -fileset constrs_1 [file join $root rtl pynq_z1.xdc]
add_files [file join $root build hello.mem]
set_property file_type {Memory Initialization Files} [get_files hello.mem]
set_property top pynq_z1_top [current_fileset]

# 基板上のPROG UARTへ出すため、PS UART0と26 bit EMIO GPIOを有効にする。
file mkdir [file join $build ip]
create_ip -name processing_system7 -vendor xilinx.com -library ip -version 5.5 \
    -module_name processing_system7_0 -dir build/hardware/ip
set_property -dict [list \
    CONFIG.PCW_CRYSTAL_PERIPHERAL_FREQMHZ {50} \
    CONFIG.PCW_UART0_PERIPHERAL_ENABLE {1} \
    CONFIG.PCW_UART0_UART0_IO {MIO 14 .. 15} \
    CONFIG.PCW_UART0_BAUD_RATE {115200} \
    CONFIG.PCW_UART_PERIPHERAL_FREQMHZ {100} \
    CONFIG.PCW_UART_PERIPHERAL_DIVISOR0 {10} \
    CONFIG.PCW_USE_M_AXI_GP0 {0} \
    CONFIG.PCW_GPIO_PERIPHERAL_ENABLE {1} \
    CONFIG.PCW_GPIO_EMIO_GPIO_ENABLE {1} \
    CONFIG.PCW_GPIO_EMIO_GPIO_WIDTH {26}] [get_ips processing_system7_0]
generate_target all [get_ips processing_system7_0]
synth_ip [get_ips processing_system7_0]

# USB-JTAG VIOコンソールIPを生成する。
create_ip -name vio -vendor xilinx.com -library ip -version 3.0 -module_name vio_0 -dir build/hardware/ip
generate_target all [get_ips vio_0]
synth_ip [get_ips vio_0]

synth_design -top pynq_z1_top -part xc7z020clg400-1
if {![info exists ::env(INDIGO_JTAG_ONLY)]} {
create_debug_core u_ila_0 ila
for {set index 1} {$index < 16} {incr index} {
    create_debug_port u_ila_0 probe
}
set_property C_DATA_DEPTH 1024 [get_debug_cores u_ila_0]
set widths {1 1 136 1 1 32 32 1 32 32 32 3 1 1 2 1}
for {set index 0} {$index < 16} {incr index} {
    set_property port_width [lindex $widths $index] \
        [get_debug_ports u_ila_0/probe$index]
}
proc exact_bus {root width} {
    set nets {}
    for {set bit 0} {$bit < $width} {incr bit} {
        lappend nets [get_nets [format {%s[%d]} $root $bit]]
    }
    return $nets
}
connect_debug_port u_ila_0/clk [get_nets cpu_clk]
connect_debug_port u_ila_0/probe0 [get_nets halted_internal]
connect_debug_port u_ila_0/probe1 [get_nets faulted_internal]
connect_debug_port u_ila_0/probe2 [exact_bus uart_history 136]
connect_debug_port u_ila_0/probe3 [get_nets hardware_done]
connect_debug_port u_ila_0/probe4 [get_nets current_privileged]
connect_debug_port u_ila_0/probe5 [exact_bus unused_debug_pc 32]
connect_debug_port u_ila_0/probe6 [exact_bus unused_debug_instruction 32]
connect_debug_port u_ila_0/probe7 [get_nets trap_valid_debug]
connect_debug_port u_ila_0/probe8 [exact_bus cause_debug 32]
connect_debug_port u_ila_0/probe9 [exact_bus epc_debug 32]
connect_debug_port u_ila_0/probe10 [exact_bus badaddr_debug 32]
connect_debug_port u_ila_0/probe11 [exact_bus interrupt_pending_debug 3]
connect_debug_port u_ila_0/probe12 [get_nets timer_interrupt_debug]
connect_debug_port u_ila_0/probe13 [get_nets external_interrupt_debug]
connect_debug_port u_ila_0/probe14 [exact_bus current_task_id_debug 2]
connect_debug_port u_ila_0/probe15 [get_nets unrecoverable_fault_debug]

}


opt_design
place_design
route_design
report_timing_summary -file [file join $build timing_summary.rpt]
report_utilization -file [file join $build utilization.rpt]
write_debug_probes -force [file join $build pynq_cpu.ltx]
write_bitstream -force [file join $build pynq_cpu.bit]
