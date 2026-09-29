/*
 * OpenVM guest layout, after `openvm-platform`'s `memory.rs` (OpenVM ships no linker script):
 *
 *   0x0000_0400  GUEST_MIN_MEM
 *                stack, grows down from STACK_TOP
 *   0x0020_0400  STACK_TOP
 *   0x0020_0800  TEXT_START: .text, then .rodata, .data, .bss
 *   _end         heap, a bump allocator growing up to GUEST_MAX_MEM
 *   0x2000_0000  GUEST_MAX_MEM (1 << 29)
 *
 * `PHDRS` keeps the ELF and program headers out of every loadable segment. Without it lld loads
 * them in a segment just below the text, which here is 0x0020_0000, inside the stack. It also
 * gives instruction-only, read-only and read-write segments, as the zkvm-standards "Static
 * Library and Linker Script" standard requires. The OpenVM runtime needs `_end` (heap start) and
 * `__global_pointer$`.
 */
OUTPUT_ARCH(riscv)
ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5);   /* R X */
    rodata PT_LOAD FLAGS(4); /* R */
    data PT_LOAD FLAGS(6);   /* R W */
}

SECTIONS
{
    . = 0x00200800;
    .text : { KEEP(*(.text._start)) *(.text .text.*) } :text
    .rodata : ALIGN(8) {
        *(.rodata .rodata.*) *(.srodata .srodata.*)
        *(.data.rel.ro .data.rel.ro.*) *(.eh_frame*) *(.gcc_except_table*)
    } :rodata
    .init_array : ALIGN(8) {
        PROVIDE(__init_array_start = .);
        KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
        KEEP(*(.init_array))
        PROVIDE(__init_array_end = .);
    } :rodata
    .data : ALIGN(8) { *(.data .data.*) } :data
    .sdata : ALIGN(8) {
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.*)
    } :data
    .sbss (NOLOAD) : ALIGN(8) { *(.sbss .sbss.*) } :data
    .bss (NOLOAD) : ALIGN(8) { *(.bss .bss.*) *(COMMON) } :data
    _end = .;

    /* Heap boundary symbols of the zkvm-standards "Static Library and Linker Script" standard. */
    _heap_start = _end;
    _heap_end = 0x20000000;
}
