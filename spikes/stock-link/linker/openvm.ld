/*
 * OpenVM guest layout, equivalent to what ere's stock-toolchain compiler gets from
 * `-Ttext=0x00200800` on lld's default layout: code from 0x0020_0800, then read-only data,
 * data and bss. The OpenVM runtime needs `_end` (heap start) and `__global_pointer$`.
 */
OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x00200800;
    .text : { KEEP(*(.text._start)) *(.text .text.*) }
    .rodata : ALIGN(8) { *(.rodata .rodata.*) *(.srodata .srodata.*) }
    .data : ALIGN(8) { *(.data .data.*) }
    .sdata : ALIGN(8) {
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.*)
    }
    .sbss (NOLOAD) : ALIGN(8) { *(.sbss .sbss.*) }
    .bss (NOLOAD) : ALIGN(8) { *(.bss .bss.*) *(COMMON) }
    _end = .;
}
