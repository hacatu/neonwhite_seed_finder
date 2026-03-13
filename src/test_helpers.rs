#![allow(unused)]

pub struct DotnetRandom {
	state: [i32; 56],
	i: usize
}

impl Default for DotnetRandom {
	fn default() -> Self {
		Self {
			state: [0; 56],
			i: 0
		}
	}
}

const MSEED: i32 = 161803398;

impl DotnetRandom {
	pub fn with_seed(seed: i32) -> Self {
		let mut res = Self::default();
		res.set_seed(seed);
		res
	}

	pub fn set_seed(&mut self, mut seed: i32) {
		seed = MSEED - seed;
		self.state[55] = seed;
		let mut num3 = 1;
		let mut index = 0;
		while index != 34 {
			index = (index + 21)%55;
			self.state[index] = num3;
			(num3, seed) = (seed-num3, num3);
			if num3 < 0 {
				num3 += i32::MAX;
			}
		}
		for _ in 0..4 {
			for k in 0..55 {
				let acc = self.state[k+1] - self.state[(k+31)%55 + 1];
				self.state[k+1] = if acc < 0 { acc + i32::MAX } else { acc };
			}
		}
	}

	pub fn next(&mut self) -> i32 {
		let i1 = if self.i + 21 >= 55 { self.i + 21 - 55 } else { self.i + 21 } + 1;
		self.i = if self.i == 55 { 1 } else { self.i + 1 };
		let mut num3 = self.state[self.i] - self.state[i1];
		if num3 < 0 { num3 += i32::MAX };
		self.state[self.i] = num3;
		num3
	}

	pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
		(self.next_double()*(max - min)as f64)as i32+min
	}

	pub fn next_double(&mut self) -> f64 {
		self.next()as f64/i32::MAX as f64
	}

	pub fn get_shuffled_idxs(num_levels: usize, seed: i32, out: &mut Vec<u8>) {
		out.clear();
		(0..num_levels as u8).collect_into(out);
		let mut random = DotnetRandom::with_seed(seed);
		for i in 0..num_levels {
			out.swap(i, random.next_range(0, num_levels as _)as _);
		}
	}
}


#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::{test_helpers::DotnetRandom, data::LEVEL_SETS, backend::get_shuffled_idxs, frontend::lookup_name};

	#[test]
	fn check_nagy() {
		let mut buf = Vec::with_capacity(96);
		let expected = "Absolution,The Clocktower,The Third Temple,Pop,Shield,Cleaner".split(",").map(|n|lookup_name(n, &LEVEL_SETS["White"]).unwrap()as u8).collect_vec();
		get_shuffled_idxs(96, 58685, &mut buf);
		assert_eq!(buf[..expected.len()], expected);
		let expected = (0..8).rev().collect_vec();
		get_shuffled_idxs(8, 121166, &mut buf);
		assert_eq!(buf[..expected.len()], expected)
	}

	#[test]
	fn check_rngs() {
		let mut buf0 = Vec::with_capacity(96);
		let mut buf1 = Vec::with_capacity(96);
		for s in 0..1000000 {
			get_shuffled_idxs(96, s, &mut buf0);
			DotnetRandom::get_shuffled_idxs(96, s, &mut buf1);
			assert_eq!(buf0, buf1);
		}
	}
}

extern crate test;

mod bench {
	#![allow(unused_imports)]
	use std::hint::black_box;
	use crate::{test_helpers::{DotnetRandom, test}, backend::get_shuffled_idxs};

	#[bench]
	fn test_linearized(b: &mut test::Bencher) {
		let mut buf = Vec::with_capacity(96);
		let mut s = 0;
		b.iter(||{
			black_box(DotnetRandom::get_shuffled_idxs(96, s, &mut buf));
			s += 1;
		});
	}

	#[bench]
	fn test_original(b: &mut test::Bencher) {
		let mut buf = Vec::with_capacity(96);
		let mut s = 0;
		b.iter(||{
			black_box(get_shuffled_idxs(96, s, &mut buf));
			s += 1;
		});
	}
}

