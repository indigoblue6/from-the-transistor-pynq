ENTRY(_start)

SECTIONS
{
    . = 0x00010000;
    .text : { *(.vectors) *(.text*) *(.rodata*) }
    .data : { *(.data*) }
    .bss : { *(.bss*) *(COMMON) }
}
