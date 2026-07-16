; Indigo32依存のリセット入口。PynqC本体は0x100以降へ配置する。
.org 0
_reset:
    li r15, 0x00010000
    li r1, trap_vector
    csrw tvec, r1
    jmp _start

.org 0x100
