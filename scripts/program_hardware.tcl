# 接続済みPYNQ-Z1へ書き込み、haltをトリガとしてILA波形を取得する。
set root [file normalize [file join [file dirname [info script]] ..]]
set build [file join $root build hardware]
open_hw_manager
connect_hw_server -allow_non_jtag
open_hw_target
set device [lindex [get_hw_devices xc7z020_1] 0]
if {$device eq ""} {
    set device [lindex [get_hw_devices xc7z020_0] 0]
}
if {$device eq ""} {
    error "PYNQ-Z1のXC7Z020デバイスが見つかりません"
}
current_hw_device $device
refresh_hw_device -update_hw_probes false $device
set_property PROGRAM.FILE [file join $build pynq_cpu.bit] $device
set_property PROBES.FILE [file join $build pynq_cpu.ltx] $device
if {![info exists ::env(INDIGO_SKIP_PROGRAM)]} {
    program_hw_devices $device
}
refresh_hw_device $device

set ila [lindex [get_hw_ilas -of_objects $device] 0]
if {$ila eq ""} {
    error "ILAコアが見つかりません"
}
set done_probe [lindex [get_hw_probes -of_objects $ila -filter {NAME =~ "*hardware_done*"}] 0]
set_property TRIGGER_COMPARE_VALUE eq1'b1 $done_probe
run_hw_ila $ila
wait_on_hw_ila $ila
set data [upload_hw_ila_data $ila]
write_hw_ila_data -force -csv_file [file join $build hardware_capture.csv] $data
puts "PYNQ-Z1実機のILAキャプチャに成功しました"
close_hw_manager
