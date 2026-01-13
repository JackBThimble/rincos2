ENTRY(kentry)

PHDRS
{
  text PT_LOAD;
  rodata PT_LOAD;
  data PT_LOAD;
}

SECTIONS {
  . = 0xffffffff80000000;

  .text : { 
    *(.text .text.*) 
  } : text

  . = ALIGN(0x1000);

  .rodata : { 
    *(.rodata .rodata.*) 
  } : rodata

  . = ALIGN(0x1000);

  .data : { 
    *(.data .data.*) 

    KEEP(*(.limine_reqs_start_marker))
    KEEP(*(.limine_reqs))
    KEEP(*(.limine_reqs_end_marker))
  } : data

  .bss : { 
    *(.bss .bss.*) 
    *(COMMON) 
  } : data 

  /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
