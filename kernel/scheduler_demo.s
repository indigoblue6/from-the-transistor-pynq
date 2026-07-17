; Kernel/User mode、syscall、timer preemption、fault isolationの統合OSデモ。
; 0x8000以上をKernel専用RAM、0x2000〜0x7fffをUser code/dataとする。
.org 0
_reset:
    li r15, 0x00010000
    li r1, trap_vector
    csrw tvec, r1

    ; 三つのtrap frameを未初期化BRAMに依存しないようゼロ化する。
    li r6, 0x00009000
    li r7, 0x00009150
zero_frames:
    store r0, [r6 + 0]
    addi r6, r6, 4
    blt r6, r7, zero_frames

    ; Task A, Task B, fault taskの初期frameを構築する。
    li r6, 0x00009000
    li r1, 0x00006000
    store r1, [r6 + 56]
    li r1, task_a
    store r1, [r6 + 60]
    movi r1, 6
    store r1, [r6 + 64]
    li r6, 0x00009080
    li r1, 0x00006800
    store r1, [r6 + 56]
    li r1, task_b
    store r1, [r6 + 60]
    movi r1, 6
    store r1, [r6 + 64]
    li r6, 0x00009100
    li r1, 0x00007000
    store r1, [r6 + 56]
    li r1, task_bad
    store r1, [r6 + 60]
    movi r1, 6
    store r1, [r6 + 64]

    ; scheduler状態: current, bad_alive, ticks。
    li r6, 0x00009200
    store r0, [r6 + 0]
    movi r1, 1
    store r1, [r6 + 4]
    store r0, [r6 + 8]

    call print_boot
    call print_user_ok

    li r1, 0x00002000
    csrw user_base, r1
    li r1, 0x00008000
    csrw user_limit, r1
    li r1, 0x0000904c
    csrw kernel_sp, r1
    csrr r1, timer_count_lo
    addi r1, r1, 1000
    csrw timer_compare_lo, r1
    movi r1, 0
    csrw timer_compare_hi, r1
    movi r1, 1
    csrw timer_control, r1
    csrw interrupt_enable, r1
    li r1, task_a
    csrw epc, r1
    movi r1, 6
    csrw status, r1
    li r15, 0x00006000
    eret

; SCRATCHでr1を退避してから、Userから到達不能な専用kernel stackへ切り替える。
trap_vector:
    csrw scratch, r1
    csrr r1, kernel_sp
    addi r1, r1, -76
    store r2, [r1 + 4]
    store r3, [r1 + 8]
    store r4, [r1 + 12]
    store r5, [r1 + 16]
    store r6, [r1 + 20]
    store r7, [r1 + 24]
    store r8, [r1 + 28]
    store r9, [r1 + 32]
    store r10, [r1 + 36]
    store r11, [r1 + 40]
    store r12, [r1 + 44]
    store r13, [r1 + 48]
    store r14, [r1 + 52]
    store r15, [r1 + 56]
    csrr r2, scratch
    store r2, [r1 + 0]
    csrr r2, epc
    store r2, [r1 + 60]
    csrr r2, status
    store r2, [r1 + 64]
    csrr r2, cause
    store r2, [r1 + 68]
    csrr r2, badaddr
    store r2, [r1 + 72]
    add r2, r1, r0
    add r15, r1, r0
    call trap_dispatch

restore_frame:
    add r15, r1, r0
    load r2, [r15 + 60]
    csrw epc, r2
    load r2, [r15 + 64]
    csrw status, r2
    load r2, [r15 + 4]
    load r3, [r15 + 8]
    load r4, [r15 + 12]
    load r5, [r15 + 16]
    load r6, [r15 + 20]
    load r7, [r15 + 24]
    load r8, [r15 + 28]
    load r9, [r15 + 32]
    load r10, [r15 + 36]
    load r11, [r15 + 40]
    load r12, [r15 + 44]
    load r13, [r15 + 48]
    load r14, [r15 + 52]
    load r1, [r15 + 0]
    load r15, [r15 + 56]
    eret

; r2=現在のtrap frame。返値r1=復帰するtaskのtrap frame。
trap_dispatch:
    li r6, 0x00009210
    store r14, [r6 + 0]
    add r12, r2, r0
    csrr r11, cause
    movi r1, 12
    beq r11, r1, handle_syscall
    li r1, 0x80000008
    beq r11, r1, handle_timer
    movi r1, 6
    beq r11, r1, handle_store_fault
    jmp kernel_panic

handle_syscall:
    load r1, [r12 + 0]
    movi r3, 1
    beq r1, r3, syscall_write
    movi r3, 2
    beq r1, r3, schedule_next
    movi r3, 3
    beq r1, r3, syscall_exit
    movi r3, 4
    beq r1, r3, syscall_ticks
    movi r1, -38
    store r1, [r12 + 0]
    jmp return_current
syscall_write:
    li r6, 0x80000000
    load r2, [r12 + 4]
    storeb r2, [r6 + 0]
    store r0, [r12 + 0]
    jmp return_current
syscall_ticks:
    li r6, 0x00009200
    load r1, [r6 + 8]
    store r1, [r12 + 0]
    jmp return_current
syscall_exit:
    li r6, 0x00009200
    load r1, [r6 + 0]
    movi r3, 2
    bne r1, r3, schedule_next
    store r0, [r6 + 4]
    jmp schedule_next

handle_timer:
    csrr r1, timer_count_lo
    addi r1, r1, 1000
    csrw timer_compare_lo, r1
    movi r1, 0
    csrw timer_compare_hi, r1
    li r6, 0x00009200
    load r1, [r6 + 8]
    addi r1, r1, 1
    store r1, [r6 + 8]
    load r3, [r6 + 4]
    bne r3, r0, schedule_next
    movi r3, 10
    blt r1, r3, schedule_next
    call print_scheduler_ok
    li r6, 0x80001000
    store r0, [r6 + 0]
    halt

handle_store_fault:
    li r6, 0x00009200
    load r1, [r6 + 0]
    movi r3, 2
    bne r1, r3, kernel_panic
    call print_trap_prefix
    load r2, [r12 + 60]
    call print_hex32
    call print_tval_prefix
    load r2, [r12 + 72]
    call print_hex32
    li r7, 0x80000000
    movi r2, 10
    storeb r2, [r7 + 0]
    call print_bad_killed
    li r6, 0x00009200
    store r0, [r6 + 4]
    store r0, [r6 + 8]
    jmp select_task0

kernel_panic:
    call print_panic
    li r6, 0x80001000
    movi r1, 1
    store r1, [r6 + 0]
    halt

schedule_next:
    li r6, 0x00009200
    load r1, [r6 + 0]
    beq r1, r0, select_task1
    movi r3, 1
    beq r1, r3, select_after_task1
    jmp select_task0
select_after_task1:
    load r3, [r6 + 4]
    bne r3, r0, select_task2
    jmp select_task0
select_task0:
    movi r1, 0
    store r1, [r6 + 0]
    li r1, 0x0000904c
    csrw kernel_sp, r1
    li r1, 0x00009000
    jmp return_selected
select_task1:
    movi r1, 1
    store r1, [r6 + 0]
    li r1, 0x000090cc
    csrw kernel_sp, r1
    li r1, 0x00009080
    jmp return_selected
select_task2:
    movi r1, 2
    store r1, [r6 + 0]
    li r1, 0x0000914c
    csrw kernel_sp, r1
    li r1, 0x00009100
    jmp return_selected
return_current:
    add r1, r12, r0
return_selected:
    li r6, 0x00009210
    load r14, [r6 + 0]
    ret

; r2の32bit値を8桁の16進数でUARTへ出す。
print_hex32:
    movi r3, 28
    movi r4, 15
    li r7, 0x80000000
print_hex_loop:
    shr r5, r2, r3
    and r5, r5, r4
    movi r6, 10
    blt r5, r6, print_hex_digit
    addi r5, r5, 87
    jmp print_hex_emit
print_hex_digit:
    addi r5, r5, 48
print_hex_emit:
    storeb r5, [r7 + 0]
    addi r3, r3, -4
    bge r3, r0, print_hex_loop
    ret

print_boot:
    li r7, 0x80000000
    movi r2, 75
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 82
    storeb r2, [r7 + 0]
    movi r2, 78
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 66
    storeb r2, [r7 + 0]
    movi r2, 79
    storeb r2, [r7 + 0]
    movi r2, 79
    storeb r2, [r7 + 0]
    movi r2, 84
    storeb r2, [r7 + 0]
    movi r2, 10
    storeb r2, [r7 + 0]
    ret

print_user_ok:
    li r7, 0x80000000
    movi r2, 85
    storeb r2, [r7 + 0]
    movi r2, 83
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 82
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 77
    storeb r2, [r7 + 0]
    movi r2, 79
    storeb r2, [r7 + 0]
    movi r2, 68
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 79
    storeb r2, [r7 + 0]
    movi r2, 75
    storeb r2, [r7 + 0]
    movi r2, 10
    storeb r2, [r7 + 0]
    ret

print_trap_prefix:
    li r7, 0x80000000
    movi r2, 84
    storeb r2, [r7 + 0]
    movi r2, 82
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 80
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 99
    storeb r2, [r7 + 0]
    movi r2, 97
    storeb r2, [r7 + 0]
    movi r2, 117
    storeb r2, [r7 + 0]
    movi r2, 115
    storeb r2, [r7 + 0]
    movi r2, 101
    storeb r2, [r7 + 0]
    movi r2, 61
    storeb r2, [r7 + 0]
    movi r2, 83
    storeb r2, [r7 + 0]
    movi r2, 84
    storeb r2, [r7 + 0]
    movi r2, 79
    storeb r2, [r7 + 0]
    movi r2, 82
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 95
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 67
    storeb r2, [r7 + 0]
    movi r2, 67
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 83
    storeb r2, [r7 + 0]
    movi r2, 83
    storeb r2, [r7 + 0]
    movi r2, 95
    storeb r2, [r7 + 0]
    movi r2, 70
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 85
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 84
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 80
    storeb r2, [r7 + 0]
    movi r2, 67
    storeb r2, [r7 + 0]
    movi r2, 61
    storeb r2, [r7 + 0]
    movi r2, 48
    storeb r2, [r7 + 0]
    movi r2, 120
    storeb r2, [r7 + 0]
    ret

print_tval_prefix:
    li r7, 0x80000000
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 84
    storeb r2, [r7 + 0]
    movi r2, 86
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 61
    storeb r2, [r7 + 0]
    movi r2, 48
    storeb r2, [r7 + 0]
    movi r2, 120
    storeb r2, [r7 + 0]
    ret

print_bad_killed:
    li r7, 0x80000000
    movi r2, 66
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 68
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 84
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 83
    storeb r2, [r7 + 0]
    movi r2, 75
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 75
    storeb r2, [r7 + 0]
    movi r2, 73
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 68
    storeb r2, [r7 + 0]
    movi r2, 10
    storeb r2, [r7 + 0]
    ret

print_scheduler_ok:
    li r7, 0x80000000
    movi r2, 83
    storeb r2, [r7 + 0]
    movi r2, 67
    storeb r2, [r7 + 0]
    movi r2, 72
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 68
    storeb r2, [r7 + 0]
    movi r2, 85
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 82
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 79
    storeb r2, [r7 + 0]
    movi r2, 75
    storeb r2, [r7 + 0]
    movi r2, 10
    storeb r2, [r7 + 0]
    ret

print_panic:
    li r7, 0x80000000
    movi r2, 75
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 82
    storeb r2, [r7 + 0]
    movi r2, 78
    storeb r2, [r7 + 0]
    movi r2, 69
    storeb r2, [r7 + 0]
    movi r2, 76
    storeb r2, [r7 + 0]
    movi r2, 32
    storeb r2, [r7 + 0]
    movi r2, 80
    storeb r2, [r7 + 0]
    movi r2, 65
    storeb r2, [r7 + 0]
    movi r2, 78
    storeb r2, [r7 + 0]
    movi r2, 73
    storeb r2, [r7 + 0]
    movi r2, 67
    storeb r2, [r7 + 0]
    movi r2, 10
    storeb r2, [r7 + 0]
    ret

.org 0x2000
; syscall ABI: r1=番号、r2-r5=引数、r1=返値。
task_a:
    movi r1, 1
    movi r2, 65
    ecall
    movi r6, 20
task_a_spin:
    addi r6, r6, -1
    bne r6, r0, task_a_spin
    jmp task_a

task_b:
    movi r1, 1
    movi r2, 66
    ecall
    movi r6, 20
task_b_spin:
    addi r6, r6, -1
    bne r6, r0, task_b_spin
    jmp task_b

task_bad:
    li r6, 0x00009000
    movi r7, 85
    store r7, [r6 + 0]
    movi r1, 3
    movi r2, 1
    ecall
    jmp task_bad
