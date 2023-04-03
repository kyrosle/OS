# `csrrw rd, csr, rs1`
# privileged instruction, reading data from CSRRS(set), and then write back to CSRRC(clean).
# rd: target register, store the data.
# csr:  CSR register.
# rs1: general register, write data into it.

# `addi rd, rs1, imm`
# rd: target register.
# rs1: source register.
# imm: A 12-digit immediate number representing the value to add.

# `sd rs2, offset(rs1)`
# rs2: source register, the value should be stored.
# offset: A 12-bit immediate number representing the number of bytes offset from the base address.
# rs1: Base address register, which holds the memory address to be stored.

# `csrr rd, csr`
# rd: target register, store the csr value.
# csr:  the csr(name/address) which we want to read

# -----------------

# Enable or disable assembly macros, using `.rept`
.altmacro
.macro SAVE_GP n
  sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
  ld x\n, \n*8(sp)
.endm
  .section .text
  .globl __alltraps
  .globl __restore
  # Address 4-byte alignment, which is required by the RISC-V privilege level specification
  .align 2
__alltraps:
  # read data from sscratch, write the current sp value into sscratch, then write the sscratch value read before into sp.
  csrrw sp, sscratch, sp # sscratch - temporary storage data.
  # now sp -> `kernel stack`, sscratch -> `user stack`
  # allocate a TrapContext on kernel stack
  # read the value from the sp, and then subtract 34*8, and then write it back to the sp.
  addi sp, sp, -34*8
  # save general-purpose registers
  sd x1, 1*8(sp)
  # skip sp(x2), we will save it later
  sd x3, 3*8(sp)
  # skip tp(x4), application does not use it
  # save x5 ~ x31
  .set n, 5
  .rept 27
      SAVE_GP %n
      .set n, n+1
  .endr
  # we can use t0/t1/t2 freely, because they were saved on kernel stack
  csrr t0, sstatus
  csrr t1, sepc
  sd t0, 32*8(sp)
  sd t1, 33*8(sp)
  # read user stack from sscratch and save it on the kernel stack
  csrr t2, sscratch
  sd t2, 2*8(sp)
  # storet the current sp(kernel stack pointer).
  mv a0, sp
  # set input argument of trap_handler(cx: &mut TrapContext)
  call trap_handler

__restore:
  # now sp -> kernel-stack(after allocated), sscratch -> user-stack
  # restore sstatus/spec
  ld t0, 32*8(sp)
  ld t1, 33*8(sp)
  ld t2, 2*8(sp)
  csrw sstatus, t0
  csrw sepc, t1
  csrw sscratch, t2
  # restore general-purpuse registers expecct sp/tp
  ld x1, 1*8(sp)
  ld x3, 3*8(sp)
  .set n, 5
  .rept 27
    LOAD_GP %n
    .set n, n + 1
  .endr
  # release TrapContext on kernel stack
  addi sp, sp, 34*8
  # now sp -> kernel-stack, sscratch -> user-stack
  csrrw sp, sscratch, sp
  sret