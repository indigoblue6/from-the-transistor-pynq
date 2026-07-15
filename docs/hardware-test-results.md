# 実機テスト記録

## 2026-07-16 PYNQ-Z1

- ツール: AMD Vivado 2025.2
- FPGA: XC7Z020-1CLG400
- 接続: Digilent USB-JTAG
- CPUクロック: 31.25 MHz
- プログラム: `examples/hello.s`
- bitstream書込み: 成功
- ILAトリガ: `halted_internal=1`で成功
- UART履歴: `Hello, PYNQ CPU!\n`と完全一致
- CPU状態: halt=1、fault=0
- 配線後タイミング: WNS 2.850 ns、TNS 0、WHS 0.035 ns、THS 0
- Block RAM: RAMB36E1 20個、RAMB18E1 1個（ILA分を含む）

外付け3.3 V USB-UART変換器はこの実行時には接続されていない。接続済みの`ttyUSB`は
PYNQ-Z1内蔵Digilent Adeptで、PLのPMODA JA1には接続されていないため、物理UART線の受信試験は
未実施である。外付け変換器を接続後、`make hardware-uart-test PORT=/dev/ttyUSBx`で追試する。
