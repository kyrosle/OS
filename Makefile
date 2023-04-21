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