# IndigoOS用: bitstreamを書き込むだけで、HALT待ちを行わない。
set root [file normalize [file join [file dirname [info script]] ..]]
set build [file join $root build hardware]
open_hw_manager
connect_hw_server -allow_non_jtag
open_hw_target
set device [lindex [get_hw_devices xc7z020_1] 0]
if {$device eq ""} { set device [lindex [get_hw_devices xc7z020_0] 0] }
if {$device eq ""} { error "PYNQ-Z1のXC7Z020デバイスが見つかりません" }
current_hw_device $device
refresh_hw_device -update_hw_probes false $device
set_property PROBES.FILE [file join $build pynq_cpu.ltx] $device
set_property PROGRAM.FILE [file join $build pynq_cpu.bit] $device
program_hw_devices $device
refresh_hw_device $device
puts "IndigoOS bitstream書込み成功。UARTシェルを使用してください。"
close_hw_manager
