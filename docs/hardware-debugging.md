# PYNQ-Z1実機デバッグ

Vivado 2025.2、XSCT、USB-JTAG、PYNQ-Z1を用いる。software/RTL確認後に次を実行する。

```bash
# 自動検証。このコマンド中はminicomを閉じる
make hardware-kernel-test PORT=/dev/ttyUSB1

# minicomを開いたまま対話表示
make hardware-kernel-console
```

targetはscheduler image、XC7Z020 synthesis/place/route/bitstream、PS UART bridge、JTAG program、
Digilent Adept内蔵PS UARTの`SCHEDULER OK`、ILA halt/faultを検証する。既定PORTは`/dev/ttyUSB1`である。
`hardware-kernel-console`を通常のデバッグ手順とし、PMODA JA1(TX)/JA2(RX)の3.3V直結UARTは代替経路とする。生成物は`build/hardware/{pynq_cpu.bit,pynq_cpu.ltx,hardware_capture.csv,
timing_summary.rpt,utilization.rpt}`である。

ILA probeはhalt、fault、UART履歴、done、mode、PC、instruction、trap valid、CAUSE、EPC、TVAL、
pending、timer IRQ、external IRQ、current task ID、unrecoverableを含む。double fault調査は
`unrecoverable=1`をtriggerとし、直前のtrap valid/CAUSE/EPC/TVALを見る。timer preemptionはtimer IRQ、
trap valid、task ID変化を同時にtriggerする。

LED0はreset解除/kernel active、LED1はrecoverable trap履歴、LED2はUser mode、LED3はunrecoverable
faultである。通常scheduler完了時はKernel HALTなのでLED0/LED1が点灯し、LED2/LED3は消灯する。

## PS UART bridgeのOCM制約

bridge ELFは低位OCMの`0x00010000`へ置き、stackは低位OCM上端内の`0x0002fff0`を使う。
`0x0003fff0`は現在のOCM低位マッピング外であり、最初の関数prologueでData Abortするため使用しない。
bridge開始時は`PS BRIDGE READY`を出力し、XSCTはprocessor reset後にELFを開始する。
