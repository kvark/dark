#[crate_id = "dark"];
#[crate_type = "bin"];
#[deny(warnings, missing_doc)];

//! Dark compressor prototype


extern mod compress;

use std::{io, num, os, vec};
use compress::{bwt, dc};
use compress::entropy::ari;


/// Coding model for BWT-DC output
pub struct DistanceModel {
	priv freq_log	: ari::FrequencyTable,
	priv freq_rest	: [ari::BinaryModel, ..4],
	/// number of distances processed
	num_processed	: uint,
}

impl DistanceModel {
	fn new(threshold: ari::Border) -> DistanceModel {
		let num_logs = 33;
		DistanceModel {
			freq_log	: ari::FrequencyTable::new_custom(num_logs, threshold, |i| {
				1<<(10 - num::min(10,i))
			}),
			freq_rest	: [ari::BinaryModel::new_flat(threshold), ..4],
			num_processed	: 0,
		}
	}

	fn encode<W: io::Writer>(&mut self, dist: dc::Distance, _sym: u8, eh: &mut ari::Encoder<W>) {
		fn int_log(d: dc::Distance) -> uint {
			let mut log = 0;
			while d>>log !=0 {log += 1;}
			log
		}
		let log = int_log(dist);
		// write exponent
		self.num_processed += 1;
		eh.encode(log, &self.freq_log).unwrap();
		self.freq_log.update(log, 10, 1);
		// write mantissa
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) & 1;
			if i >= self.freq_rest.len() {
				// just send bits past the model, equally distributed
				eh.encode(bit, self.freq_rest.last().unwrap()).unwrap();
			}else {
				let table = &mut self.freq_rest[i-1];
				eh.encode(bit, table).unwrap();
				table.update(bit, 8, 1);
			};
		}
	}

	fn decode<R: io::Reader>(&mut self, _sym: u8, dh: &mut ari::Decoder<R>) -> dc::Distance {
		self.num_processed += 1;
		let log = dh.decode(&self.freq_log).unwrap();
		self.freq_log.update(log, 10, 1);
		if log == 0 {
			return 0
		}
		let mut dist = 1 as dc::Distance;
		for i in range(1,log) {
			let bit = if i >= self.freq_rest.len() {
				dh.decode( self.freq_rest.last().unwrap() ).unwrap()
			}else {
				let table = &mut self.freq_rest[i-1];
				let bit = dh.decode(table).unwrap();
				table.update(bit, 8, 1);
				bit
			};
			dist = (dist<<1) + bit;
		}
		dist
	}
}


/// Program entry point
pub fn main() {
	let extension = &".dark";
	let input_path = match os::args() {
		[_, input, ..] => Path::new(input.clone()),
		[self_name] => {
			println!("Dark usage:");
			println!("\t{} input_file[.dark]", self_name);
			return
		},
		_ => return
	};
	let mut model = DistanceModel::new( ari::range_default_threshold >> 2 );
	let file_name = input_path.filename_str().unwrap();
	if file_name.ends_with(extension) {
		let mut in_file = match io::File::open(&input_path) {
			Ok(file) => file,
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let N = in_file.read_le_u32().unwrap() as uint;
		info!("Decoding N: {}", N);
		// create temporaries
		let mut input = vec::from_elem(N, 0u8);
		// decode alphabit
		let mut mtf = dc::MTF::new();
		let E = in_file.read_u8().unwrap() as uint;
		let mut alphabet = [0u8, ..0x100];
		let alpha_opt = if E == 0 {
			info!("Alphabet is sparse");
			None
		}else {
			in_file.read( alphabet.mut_slice_to(E) ).unwrap();
			info!("Alphabet of size {}: {:?}", E, alphabet.slice_to(E));
			Some(alphabet.slice_to(E))
		};
		// decode distances
		let mut dh = ari::Decoder::new(in_file);
		dh.start().unwrap();
		dc::decode(alpha_opt, input, &mut mtf, |sym| {
			let d = model.decode(sym, &mut dh);
			info!("Distance {} for {}", d, sym);
			Ok(d)
		}).unwrap();
		let origin = model.decode(0, &mut dh) as bwt::Suffix;
		info!("Origin: {}", origin);
		let mut suf = vec::from_elem(N, N as bwt::Suffix);
		// undo BWT and write output
		let ext_pos = file_name.len() - extension.len();
		let out_path = Path::new(format!("{}{}", file_name.slice_to(ext_pos), ".orig"));
		let mut out_file = io::File::create(&out_path);
		bwt::decode_std(input, origin, suf, |b| out_file.write_u8(b).unwrap());
	}else {
		let input = match io::File::open(&input_path).read_to_end() {
			Ok(data) => data,
			Err(e) => {
				println!("Input {} can not be read: {}", input_path.as_str(), e.to_str());
				return;
			}
		};
		let N = input.len();
		// create temporaries
		let mut output = vec::from_elem(N, 0u8);
		let mut suf = vec::from_elem(N, N as bwt::Suffix);
		// do BWT and DC
		let origin = bwt::encode_mem(input, suf, output);
		let mut mtf = dc::MTF::new();
		let dc_init = dc::encode(output, suf, &mut mtf);
		// compress to the output
		let out_path = Path::new(format!("{}{}", file_name, ".dark"));
		let mut out_file = io::File::create(&out_path).unwrap();
		info!("Encoding N: {}", N);
		out_file.write_le_u32(N as u32).unwrap();
		// encode alphabet
		let E = dc_init.len();
		let mut helper = if E > 111 {
			info!("Alphabet is sparse");
			out_file.write_u8(0).unwrap();
			let mut rd = [N as dc::Distance, ..0x100];
			for &(sym,d) in dc_init.iter() {
				rd[sym] = d;
			}
			let mut eh = ari::Encoder::new(out_file);
			for (sym,&d) in rd.iter().enumerate() {
				model.encode(d, sym as u8, &mut eh);
			}
			eh
		}else {
			info!("Alphabet of size {}", E);
			out_file.write_u8(E as u8).unwrap();
			out_file.write( dc_init.map(|&(s,_)| s) ).unwrap();
			let mut eh = ari::Encoder::new(out_file);
			for &(sym,d) in dc_init.iter() {
				info!("Init distance {} for {}", d, sym);
				model.encode(d, sym, &mut eh);
			}
			eh
		};
		// encode distances
		for (&d,&sym) in suf.iter().zip(output.iter()) {
			if d<N {
				info!("Distance {} for {}", d, sym);
				model.encode(d, sym, &mut helper);
			}
		}
		// done
		info!("Origin: {}", origin);
		model.encode(origin, 0, &mut helper);
		let (_, err) = helper.finish();
		err.unwrap();
		info!("Encoded {} distances", model.num_processed);
	}
}
