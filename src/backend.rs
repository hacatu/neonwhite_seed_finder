use std::sync::LazyLock;

use bit_set::BitSet;
use itertools::Itertools;
use anyhow::{Result as AResult, Ok as AOk};
use rayon::prelude::*;

pub const AS: [i32; 96] = [1121899819, 630111683, 1501065279, 458365203, 969558243, 1876681249, 962194431, 1077359051, 265679591, 791886952, 1582116761, 1676571504, 1476289907, 1117239683, 1503178135, 1341148412, 902714229, 1331438416, 58133212, 831516153, 285337308, 526856546, 362935496, 750214563, 210465667, 1381224997, 1846331200, 1330597961, 593162892, 1729496551, 792803163, 565661843, 863554642, 53838754, 749855384, 93067682, 1778866589, 1463507567, 367760674, 1219347826, 1648614489, 596622148, 1228675679, 243017841, 1132230640, 1891159862, 730619752, 33642253, 209795643, 283831563, 249493290, 967871855, 1560699908, 437500212, 429989927, 595043273, 267176187, 750850716, 247899536, 1735816893, 30350049, 1779080117, 484196159, 683666687, 2146567436, 1016454918, 813016862, 1422451153, 367384299, 1410110453, 1709765470, 1586690309, 963677742, 986269033, 1330385311, 1836198807, 1445664514, 119917655, 1765467570, 466789452, 650605245, 1812688947, 1120802318, 309331329, 1480003261, 1972414955, 1152445582, 426054430, 1771332474, 154812111, 1973375142, 1028015873, 1215608031, 779427428, 1188997777, 2017018019];
pub const BS: [i32; 96] = [1559595546, 1755192844, 1649316166, 1198642031, 442452829, 1200195957, 1945678308, 949569752, 2099272109, 587775847, 626863973, 1003550677, 1358625013, 1008269081, 2109153755, 65212616, 1851925803, 2137491580, 1454235444, 675580731, 1754296375, 1821177336, 2130093701, 70062080, 1503113964, 1130186590, 2005789796, 1476653312, 1174277203, 174182291, 401846963, 973512717, 638171722, 2122881600, 1380182313, 1638451829, 65271247, 818200948, 736891500, 2056119311, 1084756724, 1537539262, 255459778, 587232589, 1947978014, 1706746116, 724046315, 981848395, 315304373, 475269784, 880625662, 1543454120, 1331075398, 1047903413, 418573418, 1885901857, 1772582790, 1579254086, 1843011714, 1459749886, 1341889808, 469024996, 1922776196, 1925089818, 185928884, 1800834903, 365378955, 1383227060, 1775570415, 470701926, 2147425016, 1033724855, 1400600080, 1545599780, 1738307654, 216757113, 1565717558, 1542861112, 269567713, 1943851495, 406140275, 1023941401, 1161348939, 699007419, 1441040276, 1005876490, 1789920966, 1737751956, 1704308182, 1641764103, 2013352686, 633500808, 1122672881, 1424625261, 714229503, 615731728];

pub fn get_shuffled_idxs(num_levels: u8, seed: i32, out: &mut Vec<u8>) {
	out.clear();
	(0..num_levels).collect_into(out);
	for (i, (&a, &b)) in AS.iter().zip(&BS).take(num_levels as _).enumerate() {
		let numer = (((a as u64)*seed as u64 + (b as u64))%i32::MAX as u64)as f64;
		let j = (numer/i32::MAX as f64*num_levels as f64)as i32;
		out.swap(i, j as _)
	}
}

pub enum Rule {
	Sequence(usize, usize, Vec<u8>),
	Subset(usize, usize, BitSet)
}

impl Rule {
	pub fn len(&self) -> usize {
		match self {
			Self::Sequence(_, _, v) => v.len(),
			Self::Subset(_, _, s) => s.len()
		}
	}

	pub fn iter_domain(&self) -> impl Iterator<Item=usize> {
		match self {
			Self::Sequence(a, b, _) | Self::Subset(a, b, _) => *a..*b
		}
	}

	pub fn iter_codomain(&self) -> Box<dyn Iterator<Item=usize> + '_> {
		match self {
			Self::Sequence(_, _, v) => Box::new(v.iter().copied().map_into()),
			Self::Subset(_, _, s) => Box::new(s.iter())
		}
	}
}

/**
Returns (subset_rules, sequence_rules) where subset_rules is a vector of bytes where every 14 bytes corresponds to one entry
(a, b, 12 byte bitset), and sequence_rules is a vector of bytes where the rules are variable length: each one is a, b, followed by b-a bytes
 */
pub fn flatten_rules(rules: &[Rule]) -> (Vec<u8>, Vec<u8>) {
	let mut subset_rules = Vec::new();
	let mut sequence_rules = Vec::new();
	for rule in rules {
		match rule {
			Rule::Subset(a, b, s) => {
				subset_rules.push((*a)as _);
				subset_rules.push((*b)as _);
				for b in 0..12 {
					let mut x = 0;
					for a in 0..8 {
						if s.contains(b*8 + a) {
							x |= 1 << a;
						}
					}
					subset_rules.push(x);
				}
			},
			Rule::Sequence(a, b, v) => {
				sequence_rules.push((*a)as _);
				sequence_rules.push((*b)as _);
				sequence_rules.extend(v.iter().copied());
			}
		}
	}
	(subset_rules, sequence_rules)
}

pub fn check_shuffle_matches_flattened(buf: &[u8], subset_rules: &[u8], sequence_rules: &[u8]) -> bool {
	for i in (0..subset_rules.len()).step_by(14) {
		let mut my_set = [0u8; 12];
		let a = subset_rules[i]as usize;
		let b = subset_rules[i+1]as usize;
		for &k in &buf[a..b] {
			my_set[k as usize>>3] |= 1 << (k&7);
		}
		if !my_set.iter().zip(&subset_rules[i+2..]).all(|(&my_chunk, &subset_chunk)|subset_chunk&!my_chunk == 0) {
			return false;
		}
	}
	let mut off = 0;
	while off < sequence_rules.len() {
		let a = sequence_rules[off]as usize;
		let b = sequence_rules[off+1]as usize;
		off += 2;
		let noff = off + (b-a);
		if buf[a..b] != sequence_rules[off..noff] {
			return false;
		}
		off = noff;
	}
	true
}

pub fn find_matching_seeds_cpu(level_count: usize, result_count: usize, rules: &[Rule]) -> AResult<impl Iterator<Item=i32>> {
	let (subset_rules, sequence_rules) = flatten_rules(rules);
	AOk((0..i32::MAX).into_par_iter().map_init(||Vec::with_capacity(level_count), |buf, s|{
		get_shuffled_idxs(level_count as _, s, buf);
		(s, check_shuffle_matches_flattened(buf, &subset_rules, &sequence_rules))
	}).filter_map(|(s, p)|if p {Some(s)} else {None}).take_any(result_count).collect_vec_list().into_iter().flatten())
}

pub static LOG_FACTORIALS: LazyLock<[f64; 97]> = LazyLock::new(||{
	let mut res = [0.0; _];
	let mut i = 2;
	while i <= 96 {
		res[i] = res[i-1] + (i as f64).ln();
		i += 1;
	}
	res
});

pub fn estimate_result_count(level_count: usize, rules: &[Rule]) -> usize {
	/*
	The number of permutations that match `rules` (assuming it has no conflicts, which should have already been checked)
	is the number of ways to choose the subset of the range corresponding to the given set for every rule, times the number
	of ways to arrange each such subset, times the number of ways to arrange the free elements.

	For example,.if "The Clocktower", "The Third Temple", and "Absolution" are among the first 4 levels, and "Waterworks",
	"Movement", and "Godspeed" are among the last 4, there would be
	(4 choose 3) * 3!   *   (4 choose 3) * 3!   *   90!
	such permutations.  Note that (n choose m) * m! is just n!/(n-m)!.
	*/
	let free_elems = level_count - rules.iter().map(Rule::len).sum::<usize>();
	let log_approx_valid_perms = rules.iter().filter_map(|r|match r {
		Rule::Sequence(..) => None,
		Rule::Subset(a, b, s) => Some(LOG_FACTORIALS[(b-a)as usize] - LOG_FACTORIALS[(b-a)as usize-s.len()])
	}).sum::<f64>() + LOG_FACTORIALS[free_elems];
	(log_approx_valid_perms + ((1u32 << 31)as f64).ln() - LOG_FACTORIALS[level_count]).exp()as _
}

