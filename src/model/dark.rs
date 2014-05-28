/*!

Dark-0.51 exact compression model

# Links

http://darchiver.narod.ru/
http://code.google.com/p/adark/

*/

use std::{cmp, io};
use std::fmt::Show;
use std::vec::Vec;
use compress::entropy::ari;


/// Aggregate frequency model of two sources
pub struct Aggregate<'a,F,X,Y> {
	x: &'a X,
	y: &'a Y,
}

impl<'a, F: Float + Show, X: ari::Model<F>, Y: ari::Model<F>>
ari::Model<F> for Aggregate<'a,F,X,Y> {
	fn get_range(&self, value: F) -> (ari::Border,ari::Border) {
		let (x1,x2) = self.x.get_range(value);
		let (y1,y2) = self.y.get_range(value);
		(x1+y1, x2+y2)
	}

	fn find_value(&self, _offset: ari::Border) -> (F,ari::Border,ari::Border) {
		(NumCast::from(0).unwrap(), 0,0)	//TODO
	}

	fn get_denominator(&self) -> ari::Border {
		self.x.get_denominator() + self.y.get_denominator()
	}
}


static MAX_LOG_CODE		: uint = 8;
static MAX_LOG_CONTEXT	: uint = 11;
static NUM_LAST_LOGS	: uint = 3;
static MAX_BIT_CONTEXT	: uint = 3;
static ADAPT_POWERS: [int, ..9] = [6,5,4,3,2,1,4,6,4];


struct BinaryMultiplex {
	pub freqs: [ari::bin::Model, ..32],
}

impl BinaryMultiplex {
	fn new(threshold: ari::Border, factor: ari::Border) -> BinaryMultiplex {
		BinaryMultiplex {
			freqs: [ari::bin::Model::new_flat(threshold, factor), ..32],
		}
	}
	fn reset(&mut self) {
		for fr in self.freqs.mut_iter() {
			fr.reset_flat();
		}
	}
}


struct SymbolContext {
	pub avg_dist	: int,
	pub freq_log	: ari::table::Model,
	pub freq_extra	: BinaryMultiplex,
}

impl SymbolContext {
	fn new(threshold: ari::Border, factor: ari::Border) -> SymbolContext {
		SymbolContext{
			avg_dist	: 1000,
			freq_log	: ari::table::Model::new_flat(MAX_LOG_CODE, threshold),
			freq_extra	: BinaryMultiplex::new(threshold, factor),
		}
	}

	fn reset(&mut self) {
		self.avg_dist = 1000;
		self.freq_log.reset_flat();
		self.freq_extra.reset();
	}

	fn update(&mut self, dist: super::Distance, log_diff: int) {
		let adapt = if log_diff < -6 {7}
		else if log_diff >= 3 {3}
		else { ADAPT_POWERS[(6+log_diff) as uint] };
		self.avg_dist += (adapt*((dist as int) - self.avg_dist)) >> 3;
		debug!("\tUpdated avg_dist to {}, using raz {}, dist {} and power {}",
			self.avg_dist, log_diff, dist, adapt);
	}
}


/// Coding model for BWT-DC output
pub struct Model {
	freq_log		: Vec<Vec<ari::table::Model>>,	//[MAX_LOG_CONTEXT+1][NUM_LAST_LOGS]
	freq_log_bits	: [BinaryMultiplex, ..2],
	freq_mantissa	: [[ari::bin::Model, ..MAX_BIT_CONTEXT+1], ..32],
	/// specific context tracking
	contexts		: Vec<SymbolContext>,
	last_log_token	: uint,
	/// update parameters
	update_log_global		: uint,
	update_log_power		: uint,
	update_log_add			: ari::Border,
}

impl Model {
	/// Create a new Model instance
	pub fn new(threshold: ari::Border) -> Model {
		Model {
			freq_log		: Vec::from_fn(MAX_LOG_CONTEXT+1, |_| {
				Vec::from_fn(NUM_LAST_LOGS, |_|
					ari::table::Model::new_flat(MAX_LOG_CODE, threshold))
			}),
			freq_log_bits	: [BinaryMultiplex::new(threshold, 2), ..2],
			freq_mantissa	: [[ari::bin::Model::new_flat(threshold, 8), ..MAX_BIT_CONTEXT+1], ..32],
			contexts		: Vec::from_fn(0x100, |_| SymbolContext::new(threshold, 3)),
			last_log_token	: 1,
			update_log_global		: 12,
			update_log_power		: 5,
			update_log_add			: 5,
		}
	}

	fn int_log(d: super::Distance) -> uint {
		let mut log = 0;
		while d>>log !=0 {log += 1;}
		log
	}
}

impl super::DistanceModel for Model {
	fn new_default() -> Model {
		Model::new(ari::RANGE_DEFAULT_THRESHOLD >> 2)
	}

	fn reset(&mut self) {
		for array in self.freq_log.mut_iter() {
			for table in array.mut_iter() {
				table.reset_flat();
			}
		}
		for bm in self.freq_log_bits.mut_iter() {
			bm.reset();
		}
		for array in self.freq_mantissa.mut_iter() {
			for bm in array.mut_iter() {
				bm.reset_flat();
			}
		}
		for con in self.contexts.mut_iter() {
			con.reset();
		}
		self.last_log_token = 1;
	}

	fn encode<W: io::Writer>(&mut self, mut dist: super::Distance, ctx: &super::Context, eh: &mut ari::Encoder<W>) {
		dist += 1;
		let log = Model::int_log(dist);
		let context = self.contexts.get_mut(ctx.symbol as uint);
		let avg_log = Model::int_log(context.avg_dist as super::Distance);
		let avg_log_capped = cmp::min(MAX_LOG_CONTEXT, avg_log);
		// write exponent
		{	// base part
			let sym_freq = &mut context.freq_log;
			let log_capped = cmp::min(log, MAX_LOG_CODE)-1;
			let global_freq = self.freq_log.get_mut(avg_log_capped).get_mut(self.last_log_token);
			debug!("Dark encoding log {} with context[{}][{}] of sym {}",
				log_capped, avg_log_capped, self.last_log_token, ctx.symbol);
			eh.encode(log_capped, &ari::table::SumProxy::new(1,sym_freq, 2,global_freq, 0)).unwrap();
			sym_freq.update(log_capped, self.update_log_power, self.update_log_add);
			global_freq.update(log_capped, self.update_log_global, self.update_log_add);
		}
		if log >= MAX_LOG_CODE {	// extension
			let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==MAX_LOG_CONTEXT {1} else {0}];
			for i in range(MAX_LOG_CODE, log) {
				let bc = &mut context.freq_extra.freqs[i-MAX_LOG_CODE];
				let fc = &mut freq_log_bits.freqs[i-MAX_LOG_CODE];
				eh.encode(true, &ari::bin::SumProxy::new(1,bc, 1,fc, 1)).unwrap();
				bc.update(true);
				fc.update(true);
			}
			let i = log-MAX_LOG_CODE;
			let bc = &mut context.freq_extra.freqs[i];
			let fc = &mut freq_log_bits.freqs[i];
			eh.encode(false, &ari::bin::SumProxy::new(1,bc, 1,fc, 1)).unwrap();
			bc.update(false);
			fc.update(false);
		}
		self.last_log_token = if log<2 {0} else if log<8 {1} else {2};
		// write mantissa
		let mantissa_context = &mut self.freq_mantissa[log];
		for i in range(1,log) {
			let bit = (dist>>(log-i-1)) as uint & 1 != 0;
			if i > MAX_BIT_CONTEXT {
				// just send bits past the model, equally distributed
				eh.encode(bit, mantissa_context.last().unwrap()).unwrap();
			}else {
				let bc = &mut mantissa_context[i-1];
				eh.encode(bit, bc).unwrap();
				bc.update(bit);
			};
		}
		// update the model
		let log_diff = (log as int) - (avg_log_capped as int);	//check avg_log
		context.update(dist-1, log_diff);
	}

	fn decode<R: io::Reader>(&mut self, ctx: &super::Context, dh: &mut ari::Decoder<R>) -> super::Distance {
		let context = self.contexts.get_mut(ctx.symbol as uint);
		let avg_log = Model::int_log(context.avg_dist as super::Distance);
		let avg_log_capped = cmp::min(MAX_LOG_CONTEXT, avg_log);
		// read exponent
		let log_pre = { // base part
			let sym_freq = &mut context.freq_log;
			let global_freq = self.freq_log.get_mut(avg_log_capped).get_mut(self.last_log_token)
			;
			let log = dh.decode(&ari::table::SumProxy::new(1, sym_freq, 2, global_freq, 0)).unwrap();
			debug!("Dark decoding log {} with context[{}][{}] of sym {}",
				log, avg_log_capped, self.last_log_token, ctx.symbol);
			sym_freq.update(log, self.update_log_power, self.update_log_add);
			global_freq.update(log, self.update_log_global, self.update_log_add);
			log+1
		};
		let log = if log_pre >= MAX_LOG_CODE {	//extension
			let mut count = 0;
			let freq_log_bits = &mut self.freq_log_bits[if avg_log_capped==MAX_LOG_CONTEXT {1} else {0}];
			loop {
				let bc = &mut context.freq_extra.freqs[count];
				let fc = &mut freq_log_bits.freqs[count];
				let bit = dh.decode(&ari::bin::SumProxy::new(1,bc, 1,fc, 1)).unwrap();
				bc.update(bit);
				fc.update(bit);
				if !bit {break}
				count += 1;
			}
			log_pre + count
		}else {
			log_pre
		};
		self.last_log_token = if log<2 {0} else if log<8 {1} else {2};
		// read mantissa
		let mantissa_context = &mut self.freq_mantissa[log];
		let mut dist = 1 as super::Distance;
		for i in range(1,log) {
			let bit = if i > MAX_BIT_CONTEXT {
				dh.decode( mantissa_context.last().unwrap() ).unwrap()
			}else {
				let bc = &mut mantissa_context[i-1];
				let bit = dh.decode(bc).unwrap();
				bc.update(bit);
				bit
			};
			dist = (dist<<1) + (bit as super::Distance);
		}
		// update model
		let log_diff = (log as int) - (avg_log_capped as int);
		dist -= 1;
		context.update(dist, log_diff);
		dist
	}
}
