all: cluster

clean:
	rm cluster

cluster: src/*.rs
	rustc -O -o cluster src/main.rs

stat:	cluster
	./cluster book1.raw stat
