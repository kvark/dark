LIB_DIR		=lib/compress
LIB_PATH	=lib/libcompress-*.rlib
#TUNE		=--cfg=tune
TUNE		=
SOURCE		=Makefile src/*.rs src/model/*.rs
DEPS		=$(LIB_PATH) $(SOURCE)


.PHONY: all deps clean test test-lib bench profile test-lib

all: bin/dark

deps: $(LIB_PATH)

clean:
	rm $(LIB_PATH) bin/*


$(LIB_PATH): $(LIB_DIR)/*.rs $(LIB_DIR)/entropy/ari/*.rs
	cd lib && rustc -O -g $(TUNE) compress/lib.rs

test-lib: $(LIB_PATH)
	cd lib && rustc --test compress/lib.rs && ./compress


bin/dark: $(DEPS)
	mkdir -p bin
	rustc -O -L lib -o bin/dark $(TUNE) src/main.rs

bin/debug: $(DEPS)
	rustc -g2 -L lib -o bin/debug src/main.rs

bin/test: $(DEPS)
	rustc -O -L lib --test -o bin/test src/main.rs

bin/bench: $(DEPS)
	rustc -O -L lib --test -o bin/bench src/main.rs

bin/profile: $(DEPS)
	rustc -O -g1 -L lib -o bin/profile src/main.rs

bin/profile-saca: $(DEPS)
	rustc -O -g1 -L lib --test -o bin/profile-saca src/main.rs


test: bin/test
	bin/test

bench: bin/bench
	bin/bench --bench

pack-%: bin/dark
	bin/dark -m $* data/book1
	ls -l book1.dark
	bin/dark -m $* book1.dark
	cmp data/book1 book1.orig
	rm book1.*


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
