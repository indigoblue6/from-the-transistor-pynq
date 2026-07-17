# メモリマップ

| アドレス | 用途 | User |
|---|---|---|
| `0x00000000–0x00001fff` | Kernel instruction ROM | 不可 |
| `0x00002000–0x00003fff` | scheduler demo User code | 可 |
| `0x00004000–0x00007fff` | demo User data/stack | 可 |
| `0x00008000–0x0000ffff` | Kernel data/TCB/kernel stack | 不可 |
| `0x80000000` | UART_TX | 不可 |
| `0x80000004` | UART_STATUS | 不可 |
| `0x80000008–0x80000010` | UART RX data/status/control | 不可 |
| `0x80001000` | SIM_EXIT | 不可 |

物理実装は16 KiB instruction BRAMと48 KiB data BRAMのHarvard構成である。上表の細分は
scheduler imageの`USER_BASE=0x2000, USER_LIMIT=0x8000`による論理保護である。通常サンプルと
既存IndigoOS shellは従来配置を維持する。wordは4 byte整列、little endianである。
