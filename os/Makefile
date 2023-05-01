TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img
BOOTLOADER := ../bootloader/rustsbi-qemu.bin
APPS := ../user/src/bin/*

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

fs-img: 
	@cd ../user && make build TEST=$(TEST)
	@rm -f $(FS_IMG)
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/

run: fs-img
	@cargo build --release

	@rust-objcopy \
		--strip-all target/riscv64gc-unknown-none-elf/release/os \
		-O binary target/riscv64gc-unknown-none-elf/release/os.bin

# comamnd instructions: \
http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter1/3first-instruction-in-kernel1.html#qemu

	@qemu-system-riscv64 \
    -machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

    # -bios ../bootloader/rustsbi-qemu-new.bin \
    # -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \

rungdb:
	cargo build --release

	rust-objcopy \
		--strip-all target/riscv64gc-unknown-none-elf/release/os \
		-O binary target/riscv64gc-unknown-none-elf/release/os.bin

	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
		-s -S

gdb: 
	riscv64-unknown-elf-gdb \
    -ex 'file os/target/riscv64gc-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'
	
clean:
	cargo clean

fmt:
	cargo fmt