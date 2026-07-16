# IndigoOSをPLへ再書き込みし、PROG UARTとの双方向転送を常駐実行する。
set script_dir [file dirname [file normalize [info script]]]
set project_dir [file normalize [file join $script_dir ..]]
set bitstream [file join $project_dir build hardware pynq_cpu.bit]

if {![file exists $bitstream]} {
    error "bitstreamがありません: $bitstream"
}

targets -set -filter {name =~ "xc7z020"}
fpga -file $bitstream
targets -set -filter {name =~ "ARM Cortex-A9 MPCore #0"}

# PS UART0を115200 baud、8N1で初期化する。
mwr 0xe0000000 0x2b
mwr 0xe0000004 0x20
mwr 0xe0000018 124
mwr 0xe0000034 6
mwr 0xe0000000 0x17

# EMIO GPIO bank 2のACKおよび受信メールボックスを出力にする。
set tx_ack 0
set rx_toggle 0
set gpio_output 0
mwr 0xe000a048 $gpio_output
mwr 0xe000a284 0x01ff0200
mwr 0xe000a288 0x01ff0200

puts "IndigoOSを再書き込みしました。PROG UART双方向転送を開始します。"

while {1} {
    set mailbox [mrd -force -value 0xe000a068]

    # PL CPUからの1文字をPS UARTへ送信する。
    set tx_toggle [expr {($mailbox >> 8) & 1}]
    if {$tx_toggle != $tx_ack} {
        set value [expr {$mailbox & 0xff}]
        set uart_status [mrd -force -value 0xe000002c]
        if {($uart_status & 0x10) == 0} {
            if {$value == 10} {
                mwr 0xe0000030 13
            }
            mwr 0xe0000030 $value
            set tx_ack $tx_toggle
            set gpio_output [expr {
                ($gpio_output & ~(1 << 9)) | ($tx_ack << 9)
            }]
            mwr 0xe000a048 $gpio_output
        }
    }

    # PS UARTで受信した1文字をPL CPUへ渡す。
    set rx_ack [expr {($mailbox >> 25) & 1}]
    set uart_status [mrd -force -value 0xe000002c]
    if {(($uart_status & 0x02) == 0) && ($rx_ack == $rx_toggle)} {
        set value [mrd -force -value 0xe0000030]
        set rx_toggle [expr {$rx_toggle ^ 1}]
        set gpio_output [expr {
            ($gpio_output & ~(0x1ff << 16)) |
            (($value & 0xff) << 16) |
            ($rx_toggle << 24)
        }]
        mwr 0xe000a048 $gpio_output
    }

    after 1
}
