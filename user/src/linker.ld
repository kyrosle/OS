OUTPUT_ARCH(riscv)
ENTRY(_start)

/* Applications will be loaded and run on this physical address. */
BASE_ADDRESS = 0x80400000;

SECTIONS
{
  . = BASE_ADDRESS;
  .text : {
    *(.text.entry)
    *(.text .text.*)
  }
  .rodata : {
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
  }
  .data : {
    *(.data .data.*)
    *(.sdata .sdata.*)
  }
  .bss : {
    start_bss = .;
    *(.bss .bss.*)
    *(.sbss .sbss.*)
    end_bss = .;
  }
  /* output section */
  /* filter */
  /* discard the following segments */
  /* reducing the size of the output file */
  /DISCARD/ : {
    /* debugging information */
    *(.eh_frame)
    /* Stores debug-related symbol information that is used when code is debugged */
    *(.debug*)
  }
}