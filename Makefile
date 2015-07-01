EXE	=target/release/dark

.PHONY: all

all:
	cargo build --release

old:
	(cd etc/dark-c && make)

bin/profile: all
	rustc -O -g1 -L lib -o bin/profile src/main.rs

pack-%: all
	$(EXE) -m $* data/book1
	ls -l book1.dark
	$(EXE) -m $* book1.dark
	cmp data/book1 book1.orig
	rm book1.*

bbb:
	(cd etc/bbb && g++ main.cpp)
	etc/bbb/a.out cf data/book1 book1.bbb
	ls -l book1.bbb
	rm book1.bbb
	rm etc/bbb/a.out

compare: all old
	/usr/bin/time etc/dark-c/bin/dark p-r data/book1
	rm book1.dark
	/usr/bin/time $(EXE) -m dark data/book1
	rm book1.dark

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
