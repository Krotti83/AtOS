ENTRY(_start)

MEMORY {
    PMA (rwx) : ORIGIN = 0x0000000000080000, LENGTH = 512M
    VMA (rwx) : ORIGIN = 0xFFFFFF8000080000, LENGTH = 512M
}

STACK_SIZE = 0x4000;

SECTIONS
{
    .text : ALIGN(4096) 
    {
        *(.text.boot)
        *(.text.vectors)
        *(.text*)
    } > VMA AT > PMA

    .boot.constants : ALIGN(8)
    {
        KEEP(*(.boot.constants))
    } > VMA AT > PMA

    .rodata : ALIGN(4096) 
    { 
        *(.rodata*) 
    } > VMA AT > PMA
    
    .data : ALIGN(4096)
    { 
        *(.data*) 
    } > VMA AT > PMA
    
    .page_tables : ALIGN(4096)
    {
        _page_tables_start = .;

        KEEP(*(.page_tables.kernel.l1))
        . = ALIGN(4096);

        KEEP(*(.page_tables.kernel.l2))
        . = ALIGN(4096);

        _page_tables_end = .;
    } > VMA AT > PMA

    .bss : ALIGN(4096) 
    { 
        _bss_start = .;
        *(.bss*)
        _bss_end = .;
        *(COMMON)
    } > VMA AT > PMA

    _kernel_top = .;

    .stack (NOLOAD): ALIGN(4096) {
        _stack_bot = .;
        . = . + STACK_SIZE;
        _stack_top = .;
    } > VMA
}
