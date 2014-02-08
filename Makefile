LIB_PATH	=lib/compress

.PHONY: all test test-small

all: dark

dark: Makefile src/*.rs ${LIB_PATH}/libcompress*
	rustc -L ${LIB_PATH} -o dark src/main.rs

test-small: dark
	./dark ${LIB_PATH}/data/test.txt
	./dark test.txt.dark
	cmp ${LIB_PATH}/data/test.txt test.txt.orig
	ls -l test.txt.dark

test-large: dark
	./dark ${LIB_PATH}/data/test.large
	./dark test.large.dark
	cmp ${LIB_PATH}/data/test.large test.large.orig
	ls -l test.large.dark

test: dark
	echo -n "abracadabra" >in.dat
	./dark in.dat
	./dark in.dat.dark
	cat in.dat.orig && echo ""
	rm in.dat*
