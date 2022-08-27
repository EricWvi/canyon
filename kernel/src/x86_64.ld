ENTRY(_start)
BASE_ADDRESS = 0xffff801000000000;

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text*)
    }
    . = ALIGN(4K);
    .rodata : {
        *(.rodata*)
    }
    . = ALIGN(4K);
    .data : {
        *(.got*)
        *(.data*)
    }
    . = ALIGN(4K);
    .bss : {
        *(.bss*)
    }
    . = ALIGN(4K);
    /DISCARD/ : {
        *(.eh_frame*)
    }
}