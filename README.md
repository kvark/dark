## Dark compressor

[![Build Status](https://travis-ci.org/kvark/dark.png?branch=master)](https://travis-ci.org/kvark/dark)
```
git clone https://github.com/kvark/dark
cd dark
git submodule init
git submodule update
make
```

Dark aims to be a practical lossless universal data compressor. By combining the security of [Rust](http://rust-lang.com) with the state of art BWT implementation and compression techniques, Dark aims to be the trust-worthy tool for your day-to-day compression needs.

It requires [rust-compress](http://github.com/alexcrichton/rust-compress), and is going to be developed in cooperation with this wonderful library.
