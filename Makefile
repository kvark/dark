LIB_DIR		=lib/compress
LIB_PATH	=${LIB_DIR}/libcompress-*.rlib

.PHONY: all deps pack pack-small test-lib

all: dark


deps: ${LIB_PATH}

${LIB_PATH}: ${LIB_DIR}/*.rs ${LIB_DIR}/entropy/*.rs
	cd ${LIB_DIR} && rustc lib.rs

test-lib: ${LIB_PATH}
	cd ${LIB_DIR} && rustc --test lib.rs && ./compress


dark: Makefile src/*.rs ${LIB_PATH}
	rustc -L ${LIB_DIR} -o dark src/main.rs

release: Makefile src/*.rs ${LIB_PATH}
	rustc -O -L ${LIB_DIR} -o release src/main.rs

test: Makefile src/*.rs ${LIB_PATH}
	rustc -L ${LIB_DIR} --test -o test src/main.rs
	./test

bench: Makefile src/*.rs ${LIB_PATH}
	rustc -O -L ${LIB_DIR} --test -o bench src/main.rs
	./bench --bench

clean:
	rm ${LIB_PATH} ./dark


pack-small: dark
	./dark ${LIB_DIR}/data/test.txt
	./dark test.txt.dark
	cmp ${LIB_DIR}/data/test.txt test.txt.orig
	ls -l test.txt.dark

pack-large: dark
	./dark ${LIB_DIR}/data/test.large
	./dark test.large.dark
	cmp ${LIB_DIR}/data/test.large test.large.orig
	ls -l test.large.dark

pack: dark
	echo -n "abracadabra" >in.dat
	./dark in.dat
	./dark in.dat.dark
	cat in.dat.orig && echo ""
	rm in.dat*
