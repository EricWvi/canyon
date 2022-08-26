ENTRY(_start)
BASE_ADDRESS = 0xffff801000000000;

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text .text.*)
    }
    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
    }
    .bss : {
        *(.bss .bss.*)
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}