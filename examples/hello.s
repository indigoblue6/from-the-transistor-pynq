; UARTへ「Hello, PYNQ CPU!」を出力する最小プログラム
    li r10, 0x80000000
    movi r1, 72
    storeb r1, [r10 + 0]
    movi r1, 101
    storeb r1, [r10 + 0]
    movi r1, 108
    storeb r1, [r10 + 0]
    storeb r1, [r10 + 0]
    movi r1, 111
    storeb r1, [r10 + 0]
    movi r1, 44
    storeb r1, [r10 + 0]
    movi r1, 32
    storeb r1, [r10 + 0]
    movi r1, 80
    storeb r1, [r10 + 0]
    movi r1, 89
    storeb r1, [r10 + 0]
    movi r1, 78
    storeb r1, [r10 + 0]
    movi r1, 81
    storeb r1, [r10 + 0]
    movi r1, 32
    storeb r1, [r10 + 0]
    movi r1, 67
    storeb r1, [r10 + 0]
    movi r1, 80
    storeb r1, [r10 + 0]
    movi r1, 85
    storeb r1, [r10 + 0]
    movi r1, 33
    storeb r1, [r10 + 0]
    movi r1, 10
    storeb r1, [r10 + 0]
    halt
