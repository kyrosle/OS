build:
	@echo "Building..."
	cargo build --release

	rust-objcopy \
		--strip-all target/riscv64gc-unknown-none-elf/release/ \
		-O binary target/riscv64gc-unknown-none-elf/release/os.bin