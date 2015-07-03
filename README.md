## Dark compressor

[![Build Status](https://travis-ci.org/kvark/dark.png?branch=master)](https://travis-ci.org/kvark/dark)
[![Crate](http://meritbadge.herokuapp.com/dark)](https://crates.io/crates/dark)

Dark aims to be a practical lossless universal data compressor. By combining the security of [Rust](http://rust-lang.com) with the state of art BWT implementation and compression techniques, Dark aims to be the trust-worthy tool for your day-to-day compression needs.

It uses [rust-compress](http://github.com/alexcrichton/rust-compress), and is developed in cooperation with this library. Chunks of logic migrate into rust-compress upon stabilization (arithmetic tables, DC, soon linear BWT).

### Current status

The compressor can successfully pack and unpack any data in linear time, including the self executable. Memory consumtion is `5N` extra bytes. The following areas are being worked on:

* SACA optimization (BWT forward speed)
* Range/Binary coder optimization (pack/unpack speed)
* BWT-DC model improvements (compression ratio)

### Base line

The latest C-version of Dark-0.51 is replicated 1-to-1 here as the Dark compression model. However, due to improvements on the low level (entropy coder), the new implementation performs better (214445 vs 215505 on _book1_). The source of Dark-0.51 is also provided in `etc/dark-c/`, it was adopted to be multi-platform and includes verbose logging.
