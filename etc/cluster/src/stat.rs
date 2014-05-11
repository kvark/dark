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
	let uv = u.zip(v).fold(zero, |uv,(&a,&b)| {
		uv + (a-mu1)*(b-mv1)
	});
	uv / (num*mu2*mv2)
}

fn correlate_roll<'a, F: Float,
		U: Iterator<&'a F>,
		V: Iterator<&'a F>
		>(u: U, v: V, adapt: F) -> F {
	let (zero,one): (F,F) = (Zero::zero(), One::one());
	let (_, _, _, uu, vv, uv) = u.zip(v).fold(
		(zero,zero,zero,zero,zero,zero), |(c,u1,v1,uu,vv,uv),(&a,&b)| {
		let (du,dv) = (a-u1, b-v1);
		(c+one, u1+adapt*du, v1+adapt*dv, uu+du*du, vv+dv*dv, uv+du*dv)
	});
	//count*uv / (uu*vv)
	(one+one)*uv / (uu+vv)
}

fn correlate_both<'a, F: Float,
		U: Clone + Iterator<&'a F>,
		V: Clone + Iterator<&'a F>
		>(u: U, v: V, adapt: F) -> (F,F) {
	(correlate(u.clone(), v.clone()), correlate_roll(u,v,adapt))
}


pub fn process(values: Vec<Value>) {
	let distances: ~[f32] = values.iter().map(|v| {
		(((v.distance + 1) as f32).log2()+1.0).ln()
		}).collect();
	{
		let (_,mean,var) = variate(distances.iter());
		println!("\tMean: {}, Variance: {}", mean, var);
	}
	let adapt = 0.25f32;
	{
		let mut avg = 0.5f32;
		let predict: ~[f32] = distances.iter().map(|&d| {
			let old = avg;
			avg = adapt*d + (1.0-adapt)*avg;
			old
		}).collect();
		let (ca,cb) = correlate_both(distances.iter(), predict.iter(), adapt);
		println!("\tCorrelation(dist, predict_global) = {}\t| {}", ca, cb);
	}
	{
		let mut avg = Vec::from_elem(0x100, 0.5f32);
		let predict: ~[f32] = distances.iter().zip(values.iter()).map(|(&d,v)| {
			let v = avg.get_mut(v.symbol);
			let old = *v;
			*v = adapt*d + (1.0-adapt)*(*v);
			old
		}).collect();
		let (ca,cb) = correlate_both(distances.iter(), predict.iter(), adapt);
		println!("\tCorrelation(dist, predict_symbol) = {}\t| {}", ca, cb);
	}
	{
		let mut avg = Vec::from_elem(0x100, 0.5f32);
		let predict: ~[f32] = distances.iter().zip(values.iter()).map(|(&d,v)| {
			let v = avg.get_mut(v.last_rank);
			let old = *v;
			*v = adapt*d + (1.0-adapt)*(*v);
			old
		}).collect();
		let (ca,cb) = correlate_both(distances.iter(), predict.iter(), adapt);
		println!("\tCorrelation(dist, predict_rank) = {}\t| {}", ca, cb);
	}
}
