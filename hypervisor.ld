ENTRY(boot)

SECTIONS {
    . = 0x80200000;

    .text :{
        KEEP(*(.text.boot));
        *(.text .text.*);
        . = ALIGN(4);
        *(.text.stvec);
    }

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4) {
        *(.data .data.*);
    }

    .bss : ALIGN(4) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(4);
    . += 1024 * 1024; /* 1MB */
    __stack_top = .;

    __heap = .;
    . += 100 * 1024 * 1024; /* 100MB */
    __heap_end = .;

    /DISCARD/ : {
        *(.eh_frame);
    }
}
