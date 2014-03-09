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

### Current status

The compressor can successfully pack and unpack any data in linear time, including the self executable. The following areas are being worked on:

* SACA optimization (BWT forward speed)
* Range/Binary coder optimization (pack/unpack speed)
* BWT-DC model improvements (compression ratio)

### Base line

The latest C-version of Dark-0.51 is replicated 1-to-1 here as the Dark compression model. It narrows down _book1_ to just 214505 bytes. Source of Dark-0.51 is also included in "etc/dark-c/", it was adopted to be multi-platform and include verbose logging.