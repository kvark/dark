#![deny(missing_docs)]

//! Dark compressor prototype

extern crate byteorder;
extern crate compress;
extern crate env_logger;
extern crate getopts;
#[macro_use]
extern crate log;
extern crate num;
#[cfg(test)]
extern crate rand;

use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::{env, io};
use std::fs::File;
use std::path;
use model::DistanceModel;

/// Block encoding/decoding logic
pub mod block;
/// Compression models
pub mod model;
/// Suffix Array Construction Algorithm (SACA)
pub mod saca;

const EXTENSION: &'static str = "dark";


/// Program entry point
pub fn main() {
    env_logger::init().unwrap();
    let mut options = getopts::Options::new();
    options.optopt("m", "model", "set compression model", "dark|exp|raw|simple|ybs");
    //options.optopt("o", "output", "set output file name", "NAME");
    options.optflag("h", "help", "print this help info");

    let args: Vec<_> = env::args().collect();
    let matches = match options.parse(&args[1..]) {
        Ok(m)   => m,
        Err(f)  => panic!(f.to_string())
    };
    if matches.opt_present("h") || matches.free.is_empty() {
        let brief = format!("Dark compressor usage:\n{} [options] input_file[.dark]", args[0]);
        println!("{}", options.usage(&brief));
        return
    }

    let model = matches.opt_str("m").unwrap_or("exp".to_string());
    info!("Using model: {}", model);
    let input_path = path::Path::new(&matches.free[0]);
    let input_ext = input_path.extension();
    if input_ext.is_some() && input_ext.unwrap() == EXTENSION {
        let mut in_file = match File::open(&input_path) {
            Ok(file) => io::BufReader::new(file),
            Err(e) => {
                println!("Input {:?} can not be read: {:?}", input_path, e);
                return;
            }
        };
        let mut out_path = path::PathBuf::new();
        out_path.set_file_name(input_path.file_name().unwrap());
        out_path.set_extension("orig");
        let out_file = io::BufWriter::new(File::create(&out_path).unwrap());
        // decode the block size
        let n = in_file.read_u32::<LittleEndian>().unwrap() as usize;
        info!("Decoding N: {}", n);
        // decode the block
        let (_, _, err) = match model.as_ref() {
            "bbb"   => block::Decoder::<model::bbb::Model>      ::new(n).decode(in_file, out_file),
            "dark"  => block::Decoder::<model::dark::Model>     ::new(n).decode(in_file, out_file),
            "exp"   => block::Decoder::<model::exp::Model>      ::new(n).decode(in_file, out_file),
            "raw"   => block::Decoder::<model::RawOut>          ::new(n).decode(in_file, out_file),
            "simple"=> block::Decoder::<model::simple::Model>   ::new(n).decode(in_file, out_file),
            "ybs"   => block::Decoder::<model::ybs::Model>      ::new(n).decode(in_file, out_file),
            _       => panic!("Unknown decoding model: {}", model)
        };
        err.unwrap();
    }else {
        use std::io::Read;
        let mut input = Vec::new();
        let mut file = match File::open(&input_path) {
            Ok(f) => f,
            Err(e) => {
                println!("Input {:?} can not be read: {}", input_path, e);
                return;
            }
        };
        let n = file.read_to_end(&mut input).unwrap();
        // write the block size
        let mut out_path = path::PathBuf::new();
        out_path.set_file_name(input_path.file_name().unwrap());
        out_path.set_extension(EXTENSION);
        let mut out_file = io::BufWriter::new(File::create(&out_path).unwrap());
        info!("Encoding N: {}", n);
        out_file.write_u32::<LittleEndian>(n as u32).unwrap();
        // encode the block
        let (_, err) = match model.as_ref() {
            "bbb"   => block::Encoder::<model::bbb::Model>      ::new(n).encode(&input, out_file),
            "dark"  => block::Encoder::<model::dark::Model>     ::new(n).encode(&input, out_file),
            "exp"   => block::Encoder::<model::exp::Model>      ::new(n).encode(&input, out_file),
            "raw"   => block::Encoder::<model::RawOut>          ::new(n).encode(&input, out_file),
            "simple"=> block::Encoder::<model::simple::Model>   ::new(n).encode(&input, out_file),
            "ybs"   => block::Encoder::<model::ybs::Model>      ::new(n).encode(&input, out_file),
            _       => panic!("Unknown encoding model: {}", model)
        };
        err.unwrap();
    }
}
