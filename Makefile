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


bin/dark: Makefile src/*.rs ${LIB_PATH}
	mkdir -p bin
	rustc -O -L ${LIB_DIR} -o bin/dark src/main.rs

bin/test: Makefile src/*.rs ${LIB_PATH}
	rustc -O -L ${LIB_DIR} --test -o bin/test src/main.rs
	

bin/bench: Makefile src/*.rs ${LIB_PATH}
	rustc -O -L ${LIB_DIR} --test -o bin/bench src/main.rs

bin/profile: Makefile src/*.rs ${LIB_PATH}
	rustc -O -g -L ${LIB_DIR} -o bin/profile src/main.rs

bin/profile-saca: Makefile src/*.rs ${LIB_PATH}
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


pack-small: bin/dark
	bin/dark ${LIB_DIR}/data/test.txt
	bin/dark test.txt.dark
	cmp ${LIB_DIR}/data/test.txt test.txt.orig
	ls -l test.txt.*
	rm test.txt.*

pack-large: bin/dark
	bin/dark ${LIB_DIR}/data/test.large
	bin/dark test.large.dark
	cmp ${LIB_DIR}/data/test.large test.large.orig
	ls -l test.large.*
	rm test.large.*

pack: bin/dark
	echo -n "abracadabra" >in.dat
	bin/dark in.dat
	bin/dark in.dat.dark
	cat in.dat.orig && echo ""
	rm in.dat*
