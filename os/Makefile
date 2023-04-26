run:
	cargo build --release

	rust-objcopy \
		--strip-all target/riscv64gc-unknown-none-elf/release/os \
		-O binary target/riscv64gc-unknown-none-elf/release/os.bin

# comamnd instructions: \
http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter1/3first-instruction-in-kernel1.html#qemu
	qemu-system-riscv64 \
    -machine virt \
		-nographic \
    -bios ../bootloader/rustsbi-qemu-new.bin \
    -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \

rungdb:
	cargo build --release

	rust-objcopy \
		--strip-all target/riscv64gc-unknown-none-elf/release/os \
		-O binary target/riscv64gc-unknown-none-elf/release/os.bin

	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu-new.bin \
		-device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \
		-s -S

gdb: 
	riscv64-unknown-elf-gdb \
    -ex 'file os/target/riscv64gc-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'
	
clean:
	cargo clean