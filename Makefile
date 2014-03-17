LIB_DIR		=lib/compress
LIB_PATH	=lib/libcompress-*.rlib
#TUNE		=--cfg=tune
TUNE		=

.PHONY: all deps clean test test-lib bench profile pack pack-small test-lib

all: bin/dark

deps: ${LIB_PATH}

clean:
	rm ${LIB_PATH} bin/*


${LIB_PATH}: ${LIB_DIR}/*.rs ${LIB_DIR}/entropy/*.rs
	cd lib && rustc -O -g ${TUNE} compress/lib.rs

test-lib: ${LIB_PATH}
	cd lib && rustc --test compress/lib.rs && ./compress


bin/dark: Makefile src/*.rs src/model/*.rs ${LIB_PATH}
	mkdir -p bin
	rustc -O -L lib -o bin/dark ${TUNE} src/main.rs

bin/debug: bin/dark
	rustc -g2 -L lib -o bin/debug src/main.rs

bin/test: bin/dark
	rustc -O -L lib --test -A deprecated_owned_vector -o bin/test src/main.rs
	
bin/bench: bin/dark
	rustc -O -L lib --test -o bin/bench src/main.rs

bin/profile: bin/dark
	rustc -O -g1 -L lib -o bin/profile src/main.rs

bin/profile-saca: bin/dark
	rustc -O -g1 -L lib --test -o bin/profile-saca src/main.rs


test: bin/test
	bin/test

bench: bin/bench
	bin/bench --bench

profile-saca: etc/callgrind/saca.out
profile: etc/callgrind/dark.out

etc/callgrind/saca.out: bin/profile-saca
	valgrind --tool=callgrind bin/profile-saca --bench
	mkdir -p etc/callgrind
	mv callgrind.out.* etc/callgrind/saca.out

etc/callgrind/dark.out: bin/profile
	valgrind --tool=callgrind bin/profile lib/compress/data/test.large
	mkdir -p etc/callgrind
	mv callgrind.out.* etc/callgrind/dark.out
	ls -l test.large.dark
	rm test.large.dark


pack: bin/dark
	bin/dark data/book1
	ls -l book1.dark
	bin/dark book1.dark
	cmp data/book1 book1.orig
	rm book1.*
