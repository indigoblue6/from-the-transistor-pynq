# bitstreamを書き込み、PSを初期化してPROG UARTブリッジをCortex-A9で起動する。
set root [file normalize [file join [file dirname [info script]] ..]]
set build [file join $root build hardware]
connect
targets -set -nocase -filter {name =~ "APU*"}
rst -system
after 500
source [file join $build ip processing_system7_0 ps7_init.tcl]
ps7_init
fpga -file [file join $build pynq_cpu.bit]
ps7_post_config
targets -set -nocase -filter {name =~ "*Cortex-A9*#1"}
stop
targets -set -nocase -filter {name =~ "*Cortex-A9*#0"}
stop
# Boot ROMや例外handlerのモードを引き継がず、SVC modeからbridgeを開始する。
rst -processor
dow [file join $root build ps_uart_bridge bridge.elf]
con
after 1500
disconnect
