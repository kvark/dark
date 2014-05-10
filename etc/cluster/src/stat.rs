//! Statistical data

use std::num::{Float, One, Zero};
use std::vec::Vec;
use super::Value;


fn variate<'a, F: Float,
		I: Clone + Iterator<&'a F>
		>(mut it: I) -> (F,F,F) {
	let zero: F = Zero::zero();
	let (s0,s1,s2) = it.fold((zero,zero,zero), |(s0,s1,s2),&v| {
		(s0+One::one(), s1+v, s2+v*v)
	});
	let m1 = s1/s0;
	let m2 = (s2/s0 - m1*m1).sqrt();
	(s0,m1,m2)
}

fn correlate<'a, F: Float,
		U: Clone + Iterator<&'a F>,
		V: Clone + Iterator<&'a F>
		>(u: U, v: V) -> F {
	let (num,mu1,mu2) = variate(u.clone());
	let (_,  mv1,mv2) = variate(v.clone());
	let zero: F = Zero::zero();
	let x = u.zip(v).fold(zero, |x,(&a,&b)| {
		x + (a-mu1)*(b-mv1)
	});
	x / (num*mu2*mv2)
}


pub fn process(values: Vec<Value>) {
	let distances: ~[f32] = values.iter().map(|v| {
		(((v.distance + 1) as f32).log2()+1.0).ln()
		}).collect();
	{
		let (_,mean,var) = variate(distances.iter());
		println!("\tMean: {}, Variance: {}", mean, var);
	}
	{
		let mut avg = 0.5f32;
		let weight = 0.3f32;
		let predict: ~[f32] = distances.iter().map(|&d| {
			let old = avg;
			avg = weight*d + (1.0-weight)*avg;
			old
		}).collect();
		let corel = correlate(distances.iter(), predict.iter());
		println!("\tCorrelation(dist, predict_global) = {}", corel);
	}
	{
		let mut avg = Vec::from_elem(0x100, 0.5f32);
		let weight = 0.3f32;
		let predict: ~[f32] = distances.iter().zip(values.iter()).map(|(&d,v)| {
			let v = avg.get_mut(v.symbol);
			let old = *v;
			*v = weight*d + (1.0-weight)*(*v);
			old
		}).collect();
		let corel = correlate(distances.iter(), predict.iter());
		println!("\tCorrelation(dist, predict_symbol) = {}", corel);
	}
	{
		let mut avg = Vec::from_elem(0x100, 0.5f32);
		let weight = 0.3f32;
		let predict: ~[f32] = distances.iter().zip(values.iter()).map(|(&d,v)| {
			let v = avg.get_mut(v.last_rank);
			let old = *v;
			*v = weight*d + (1.0-weight)*(*v);
			old
		}).collect();
		let corel = correlate(distances.iter(), predict.iter());
		println!("\tCorrelation(dist, predict_rank) = {}", corel);
	}
}
