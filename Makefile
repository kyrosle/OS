run:
	make -C user build
	make -C os run

gdb:
	make -C user build
	make -C os gdb

clean: 
	make -C user clean
	make -C os clean