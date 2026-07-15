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

synth_design -top pynq_z1_top -part xc7z020clg400-1
create_debug_core u_ila_0 ila
create_debug_port u_ila_0 probe
create_debug_port u_ila_0 probe
create_debug_port u_ila_0 probe
set_property C_DATA_DEPTH 1024 [get_debug_cores u_ila_0]
set_property port_width 1 [get_debug_ports u_ila_0/probe0]
set_property port_width 1 [get_debug_ports u_ila_0/probe1]
set_property port_width 137 [get_debug_ports u_ila_0/probe2]
set_property port_width 1 [get_debug_ports u_ila_0/probe3]
connect_debug_port u_ila_0/clk [get_nets cpu_clk]
connect_debug_port u_ila_0/probe0 [get_nets halted_internal]
connect_debug_port u_ila_0/probe1 [get_nets faulted_internal]
set history_nets [get_nets uart_history*]
set history_nets [lsearch -all -inline -not -exact $history_nets uart_history]
connect_debug_port u_ila_0/probe2 $history_nets
connect_debug_port u_ila_0/probe3 [get_nets hardware_done]

opt_design
place_design
route_design
report_timing_summary -file [file join $build timing_summary.rpt]
report_utilization -file [file join $build utilization.rpt]
write_debug_probes -force [file join $build pynq_cpu.ltx]
write_bitstream -force [file join $build pynq_cpu.bit]
