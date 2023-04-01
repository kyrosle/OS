/* target is risve */
OUTPUT_ARCH(riscv) 
/* the entry point, global symbol */
ENTRY(_start)
/* the kerenl initialize code address */
BASE_ADDRESS = 0x80200000;

SECTIONS
{
  /* current address set as BASE_ADDRESS */
  . = BASE_ADDRESS;
  /* the start address of kernel */
  skernel = .;

  /* aligin the .text area, recording the start of text and the end of text  */
  stext = .;
  .text : {
    *(.text.entry)
    *(.text .text.*)
  }

  . = ALIGN(4K);
  etext = .;
  /* --------- */

  /* aligin the .rodata area  */
  srodata = .;
  .rodata : {
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
  }
  . = ALIGN(4K);
  erodata = .;
  /* --------- */

  /* aligin the .data area  */
  sdata = .;
  .data : {
    *(.data .data.*)
    *(.sdata .sdata.*)
  }
  . = ALIGN(4K);
  edata = .;
  /* --------- */

  /* aligin the bss area */
  .bss : {
    *(.bss.stack)
    sbss = .;
    *(.bss .bss.*)
    *(.sbss .sbss.*)
  }
  . = ALIGN(4K);
  ebss = .;
  /* --------- */

  /* the end of kernel address */
  ekernel = .;

   /* output section */
  /* filter */
  /* discard the following segments */
  /* reducing the size of the output file */
  /DISCARD/ : {
    /* debugging information */
    *(.eh_frame)
  }
}