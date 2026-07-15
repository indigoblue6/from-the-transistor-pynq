# PYNQ-Z1の125 MHz PLクロック
set_property -dict { PACKAGE_PIN H16 IOSTANDARD LVCMOS33 } [get_ports clk_125mhz]
create_clock -add -name sys_clk -period 8.000 -waveform {0.000 4.000} [get_ports clk_125mhz]

# BTN0、LED0～LED3
set_property -dict { PACKAGE_PIN D19 IOSTANDARD LVCMOS33 } [get_ports reset_button]
set_property -dict { PACKAGE_PIN R14 IOSTANDARD LVCMOS33 } [get_ports {led[0]}]
set_property -dict { PACKAGE_PIN P14 IOSTANDARD LVCMOS33 } [get_ports {led[1]}]
set_property -dict { PACKAGE_PIN N16 IOSTANDARD LVCMOS33 } [get_ports {led[2]}]
set_property -dict { PACKAGE_PIN M14 IOSTANDARD LVCMOS33 } [get_ports {led[3]}]

# PMODA JA1へ115200 baud、8N1のUART TXを出力する。
set_property -dict { PACKAGE_PIN Y18 IOSTANDARD LVCMOS33 DRIVE 8 SLEW SLOW } [get_ports uart_tx_pin]

set_property CFGBVS VCCO [current_design]
set_property CONFIG_VOLTAGE 3.3 [current_design]
