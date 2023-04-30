run:
	make -C user build
	make -C os run

gdbr:
	make -C os rungdb
gdbs:
	make -C os gdb

clean: 
	make -C user clean
	make -C os clean
	make -C easy-fs clean
	make -C easy-fs-fuse clean

fmt: 
	make -C user fmt
	make -C os fmt
	make -C easy-fs fmt
	make -C easy-fs-fuse fmt