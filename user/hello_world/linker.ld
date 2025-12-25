ENTRY(_start)

SECTIONS
{
    . = 0x500000;
    
    .text : {
        *(.text .text.*)
    }
    
    .rodata : {
        *(.rodata .rodata.*)
    }
    
    .data : {
        *(.data .data.*)
    }
    
    .bss : {
        *(.bss .bss.*)
    }
    
    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
    }
}