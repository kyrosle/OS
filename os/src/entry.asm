# we want to put all below into the segment named of `.text.entry`, 
# it can be teh entry point of kernel(at a lower address).
  .section .text.entry 
  .global _start # we declare the symbol `_start` is global, it can used by other files.
# declare a symbol
_start:
  la sp, boot_stack_top
  call rust_main

  .section .bss.stack
  .global boot_stack_lower_bound
boot_stack_lower_bound:
  .space 4096 * 16 # 64 KiB space for application stack space.
  .global boot_stack_top
boot_stack_top: