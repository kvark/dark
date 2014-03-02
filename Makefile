LIB_DIR		=lib/compress
LIB_PATH	=${LIB_DIR}/libcompress-*.rlib

.PHONY: all deps clean test test-lib bench profile pack pack-small test-lib

all: bin/dark


deps: ${LIB_PATH}

clean:
	rm ${LIB_PATH} bin/*

${LIB_PATH}: ${LIB_DIR}/*.rs ${LIB_DIR}/entropy/*.rs
	cd ${LIB_DIR} && rustc -O -g lib.rs

test-lib: ${LIB_PATH}
	cd ${LIB_DIR} && rustc --test lib.rs && ./compress


bin/dark: Makefile src/*.rs src/model/*.rs ${LIB_PATH}
	mkdir -p bin
	rustc -O -L ${LIB_DIR} -o bin/dark src/main.rs

bin/test: bin/dark
	rustc -O -L ${LIB_DIR} --test -o bin/test src/main.rs
	

bin/bench: bin/dark
	rustc -O -L ${LIB_DIR} --test -o bin/bench src/main.rs

bin/profile: bin/dark
	rustc -O -g -L ${LIB_DIR} -o bin/profile src/main.rs

bin/profile-saca: bin/dark
	rustc -O -g -L ${LIB_DIR} --test -o bin/profile-saca src/main.rs


test: bin/test
	bin/test

bench: bin/bench
	bin/bench --bench

profile-saca: callgrind.saca
profile: callgrind.dark

callgrind.saca: bin/profile-saca
	valgrind --tool=callgrind bin/profile-saca --bench
	mv callgrind.out.* callgrind.saca.out

callgrind.dark: bin/profile
	valgrind --tool=callgrind bin/profile lib/compress/data/test.large
	mv callgrind.out.* callgrind.dark.out
	ls -l test.large.dark
	rm test.large.dark


pack: bin/dark
	bin/dark data/book1
	ls -l book1.dark
	bin/dark book1.dark
	cmp data/book1 book1.orig
	rm book1.*
