use bit_set::BitSet;
use itertools::Itertools;
use anyhow::{Result as AResult, Ok as AOk, bail};
use crate::{
	backend::Rule,
	data::{ALL_LEVELS, ALTERNATE_NAMES, LEVEL_SETS}
};

pub fn is_abbreviation(cleanedname: &str, fullname: &str) -> bool {
	let _fullname = fullname.to_lowercase();
	let mut full_chunks = _fullname.split_whitespace().peekable();
	for mut chu in cleanedname.split_whitespace() {
		loop {
			if chu.len() == 0 {
				break;
			}
			if full_chunks.peek().is_none() {
				return false;
			}
			let l = chu.chars().zip(full_chunks.peek().unwrap().chars()).take_while(|(a,b)|a==b).count();
			if l != 0 {
				(_, chu) = chu.split_at(l);
				full_chunks.peek_mut().map(|r|(_, *r) = r.split_at(l));
			}
			let _ = full_chunks.next();
		}
	}
	true
}

pub fn lookup_name(name: &str, level_set: &BitSet) -> AResult<usize> {
	let mut res = None;
	let cleanedname = name.to_lowercase().replace("1", "i").replace("2", "ii");
	let mut j = 0;
	let mut actual_res = 0;
	for (i, fullname) in ALL_LEVELS.iter().copied().enumerate() {
		if !level_set.contains(i) { continue }
		if !is_abbreviation(cleanedname.as_str(), fullname) {
			j += 1;
			continue;
		}
		if res.is_some() {
			bail!("Level name \"{name}\" is ambigious and could match \"{}\" or \"{fullname}\"", ALL_LEVELS[res.unwrap()]);
		}
		res = Some(i);
		actual_res = j;
		j += 1;
	}
	AOk(match res {
		Some(_) => actual_res,
		None => {
			for &(fullname, i) in ALTERNATE_NAMES.iter() {
				if !level_set.contains(i) || !is_abbreviation(cleanedname.as_str(), fullname) {
					continue;
				}
				if res.is_some() {
					bail!("Level name \"{name}\" matches no proper levels, but abbreviations for both \"{}\" and \"{}\"", ALL_LEVELS[res.unwrap()], ALL_LEVELS[i]);
				}
				res = Some(i);
			}
			let Some(res) = res else {
				bail!("Level name \"{name}\" did not match anything");
			};
			level_set.iter().filter(|&i|i<res).count()
		}
	})
}

pub fn guess_rush_from_abbr(rush_abbr: &[String]) -> AResult<(&'static str, &'static BitSet)> {
	let cleanedname = rush_abbr.into_iter().map(|s|s.to_lowercase()).join(" ");
	if cleanedname.trim().len() == 0 {
		return AOk(("White", &LEVEL_SETS["White"]));
	}
	if let Ok(idx) = i32::from_str_radix(&cleanedname.replace(|s: char|s.is_whitespace(), ""), 10) {
		const KEYS: [&'static str; 16] = [
			"White",
			"Rebirth",
			"Killer Inside",
			"Only Shallow",
			"The Old City",
			"The Burn That Cures",
			"Covenant",
			"Reckoning",
			"Benediction",
			"Apocrypha",
			"The Third Temple",
			"Thousand Pound Butterfly",
			"Hand of God",
			"Red",
			"Violet",
			"Yellow"
		];
		if 0 <= idx && idx < KEYS.len() as i32 {
			let name = KEYS[idx as usize];
			return AOk((name, &LEVEL_SETS[name]));
		}
		if idx == 1000 {
			let name = "Thousand Pound Butterfly";
			return AOk((name, &LEVEL_SETS[name]));
		}
	}
	let mut res: Option<(&str, &BitSet)> = None;
	for (&fullname, level_set) in LEVEL_SETS.iter() {
		if is_abbreviation(cleanedname.trim(), fullname) {
			if res.is_some() {
				bail!("Rush abbreviation \"{cleanedname}\" is ambiguous and could match {} or {fullname}", res.unwrap().0);
			}
			res = Some((fullname, level_set));
		}
	}
	let Some(res) = res else {
		bail!("Rush abbreviation \"{cleanedname}\" did not match anything");
	};
	AOk(res)
}

pub fn guess_rule_once(description: &str, level_set: &BitSet) -> AResult<Rule> {
	let Some((a_chu, _description)) = description.split_once(":") else {
		bail!("The rule description \"{description}\" is not formatted correctly (no \":\" found) (should be \"INDEX?:INDEX? \"=\"|\">\" ID (\",\" ID)*\")");
	};
	let (b_chu, _description, is_seq) =
		if let Some((b_chu, _description)) = _description.split_once("=") {
			(b_chu, _description, true)
		} else if let Some((b_chu, _description)) = _description.split_once(">") {
			(b_chu, _description, false)
		} else {
			bail!("The rule description \"{description}\" is not formatted correctly (no \"=\" or \">\" found) (should be \"INDEX?:INDEX? \"=\"|\">\" ID (\",\" ID)*\")");
		};
	let parse_index = |s|AOk(match i32::from_str_radix(s, 10) {
		Ok(i) => Some(
			if i < 0 {
				if i >= -(level_set.len()as i32) {
					i + level_set.len()as i32
				} else {
					bail!("The index \"{i}\" is out of range (it should be relative to the length of the rush)");
				}
			} else if i < level_set.len()as i32 {
				i
			} else {
				bail!("The index \"{i}\" is out of range (it should be relative to the length of the rush)");
			}as usize
		),
		Err(_) => None
	});
	let level_ids = _description.split(",").map(|s|{
		let s = s.trim();
		AOk(match parse_index(s)? {
			Some(i) => i,
			None => lookup_name(s, level_set)?
		})
	});
	let mut res = if is_seq {
		Rule::Sequence(0, 0, level_ids.map_ok(|i|i as _).try_collect()?)
	} else {
		let mut s = BitSet::with_capacity(level_set.capacity());
		for i in level_ids { // I'm not sure how to ergonomically deal with iterators of Result
			s.insert(i?);
		}
		Rule::Subset(0, 0, s)
	};
	if res.len() == 0 {
		bail!("Empty level list for rule \"{description}\"");
	}
	let a_chu = a_chu.trim();
	let b_chu = b_chu.trim();
	let guess_index = |s|AOk(match parse_index(s)? {
		Some(i) => i,
		None => {
			bail!("Could not parse index \"{s}\"");
		}
	});
	let mut a = if a_chu.len() == 0 { 0 } else { guess_index(a_chu)? };
	let mut b = if b_chu.len() == 0 { level_set.len() } else { guess_index(b_chu)? };
	if b <= a || b-a < res.len() {
		bail!("Range \"{a_chu}:{b_chu}\" is too small to fit the requested levels");
	}
	if is_seq && b-a > res.len() {
		if b_chu.len() == 0 {
			b -= (b-a) - res.len()
		} else if a_chu.len() == 0 {
			a += (b-a) - res.len()
		} else {
			bail!("Range \"{a_chu}:{b_chu}\" is too large to fit the requested sequence");
		}
	}
	match &mut res {
		Rule::Sequence(res_a, res_b, _) | Rule::Subset(res_a, res_b, _) => (*res_a, *res_b) = (a, b)
	}
	AOk(res)
}

pub fn check_rules(rules: &[Rule], level_set: &BitSet) -> AResult<()> {
	let mut domains = vec![None; level_set.len()];
	let mut codomains = vec![None; level_set.len()];
	for (i, rule) in rules.iter().enumerate() {
		for j in rule.iter_domain() {
			match &mut domains[j] {
				Some(i0) => {
					bail!("Rules {i0} and {i} overlap and both affect index {j} of the desired shuffle");
				},
				s @ None => *s = Some(i)
			}
		}
		for j in rule.iter_codomain() {
			match &mut codomains[j] {
				Some(i0) => {
					bail!("Rules {i0} and {i} overlap and both include level {j} of the rush ({})", ALL_LEVELS[level_set.iter().nth(j).unwrap()]);
				},
				s @ None => *s = Some(i)
			}
		}
	}
	AOk(())
}

pub fn guess_rules_from_description(description: &[String], level_set: &BitSet) -> AResult<Vec<Rule>> {
	let description = description.join(" ");
	let res: Vec<_> = description.split("&").map(|s|guess_rule_once(s, level_set)).try_collect()?;
	check_rules(&res, level_set)?;
	AOk(res)
}

