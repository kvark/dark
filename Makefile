EXE	=target/release/dark

.PHONY: all

all:
	cargo build --release

old:
	cd etc/dark-c && make

bin/profile: all
	rustc -O -g1 -L lib -o bin/profile src/main.rs

pack-%: all
	$(EXE) -m $* data/book1
	ls -l book1.dark
	$(EXE) -m $* book1.dark
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
