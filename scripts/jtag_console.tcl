# USB-JTAG VIOをバイトストリーム端末として使うIndigoOSコンソール。
# PS UARTや外付けUSB-UARTは必要ない。
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
set vio [lindex [get_hw_vios -of_objects $device] 0]
if {$vio eq ""} { error "JTAG VIOコアが見つかりません。OS用bitstreamを生成してください" }
set in_probe [lindex [get_hw_probes -of_objects $vio -filter {NAME =~ "*probe_out0"}] 0]
set out_probe [lindex [get_hw_probes -of_objects $vio -filter {NAME =~ "*probe_in0"}] 0]
set rx_toggle 0
set tx_ack 0
set out_value 0
proc vio_write {probe value} {
    set_property OUTPUT_VALUE [format "0x%03x" $value] $probe
    commit_hw_vio [get_hw_vios]
}
proc vio_read {vio probe} {
    refresh_hw_vio $vio
    set raw [get_property INPUT_VALUE $probe]
    scan $raw %i value
    return $value
}
fconfigure stdin -blocking 0 -buffering none -translation binary
puts "IndigoOS JTAGコンソール開始（Ctrl-Dで終了）"
flush stdout
while {1} {
    set ch [read stdin 1]
    if {[string length $ch] != 0} {
        binary scan $ch c byte
        set byte [expr {$byte & 255}]
        set rx_toggle [expr {$rx_toggle ^ 1}]
        set value [expr {($rx_toggle << 9) | ($tx_ack << 8) | $byte}]
        vio_write $in_probe $value
    }
    set out_value [vio_read $vio $out_probe]
    if {($out_value & 0x100) != 0} {
        puts -nonewline [format %c [expr {$out_value & 255}]]
        flush stdout
        set tx_ack [expr {$tx_ack ^ 1}]
        set value [expr {($rx_toggle << 9) | ($tx_ack << 8) | ($byte & 255)}]
        vio_write $in_probe $value
    }
    if {[eof stdin]} { break }
    after 5
}
close_hw_manager
