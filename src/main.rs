#![feature(iter_collect_into, test)]
use std::{collections::HashMap, iter, sync::LazyLock};
use bit_set::BitSet;
use itertools::Itertools;
use rayon::prelude::*;
use ocl::ProQue;
use anyhow::{Result as AResult, Ok as AOk, bail};

static ALL_LEVELS: LazyLock<Vec<&'static str>> = LazyLock::new(||vec![
	"Movement",
	"Pummel",
	"Gunner",
	"Cascade",
	"Elevate",
	"Bounce",
	"Purify",
	"Climb",
	"Fasttrack",
	"Glass Port",
	"Take Flight",
	"Godspeed",
	"Dasher",
	"Thrasher",
	"Outstretched",
	"Smackdown",
	"Catwalk",
	"Fastlane",
	"Distinguish",
	"Dancer",
	"Guardian",
	"Stomp",
	"Jumper",
	"Dash Tower",
	"Descent",
	"Driller",
	"Canals",
	"Sprint",
	"Mountain",
	"Superkinetic",
	"Arrival",
	"Forgotten City",
	"The Clocktower",
	"Fireball",
	"Ringer",
	"Cleaner",
	"Warehouse",
	"Boom",
	"Streets",
	"Steps",
	"Demolition",
	"Arcs",
	"Apartment",
	"Hanging Gardens",
	"Tangled",
	"Waterworks",
	"Killswitch",
	"Falling",
	"Shocker",
	"Bouquet",
	"Prepare",
	"Triptrack",
	"Race",
	"Bubble",
	"Shield",
	"Overlook",
	"Pop",
	"Minefield",
	"Mimic",
	"Trigger",
	"Greenhouse",
	"Sweep",
	"Fuse",
	"Heaven's Edge",
	"Zipline",
	"Swing",
	"Chute",
	"Crash",
	"Ascent",
	"Straightaway",
	"Firecracker",
	"Streak",
	"Mirror",
	"Escalation",
	"Bolt",
	"Godstreak",
	"Plunge",
	"Mayhem",
	"Barrage",
	"Estate",
	"Trapwire",
	"Ricochet",
	"Fortress",
	"Holy Ground",
	"The Third Temple",
	"Spree",
	"Breakthrough",
	"Glide",
	"Closer",
	"Hike",
	"Switch",
	"Access",
	"Congregation",
	"Sequence",
	"Marathon",
	"Sacrifice",
	"Absolution",
	"Elevate Traversal I",
	"Elevate Traversal II",
	"Purify Traversal",
	"Godspeed Traversal",
	"Stomp Traversal",
	"Fireball Traversal",
	"Dominion Traversal",
	"Book of Life Traversal",
	"Doghouse",
	"Choker",
	"Chain",
	"Hellevator",
	"Razor",
	"All Seeing Eye",
	"Resident Saw I",
	"Resident Saw II",
	"Sunset Flip Powerbomb",
	"Balloon Mountain",
	"Climbing Gym",
	"Fisherman Suplex",
	"STF",
	"Arena",
	"Attitude Adjustment",
	"Rocket",
]);

static ALTERNATE_NAMES: LazyLock<Vec<(&'static str, usize)>> = LazyLock::new(||vec![
	("Fisherman Souplex", 97 + 8 + 8 + 3),
	("Special Task Force", 97 + 8 + 8 + 4)
]);

static LEVEL_SETS: LazyLock<HashMap<&'static str, BitSet>> = LazyLock::new(||HashMap::from([
	("White", BitSet::from_iter((0..95).chain(iter::once(96)))),
	("Mikey", BitSet::from_iter((0..95).chain(iter::once(96)))),
	("Red", BitSet::from_iter(97..97+8)),
	("Violet", BitSet::from_iter(97+8..97+8+8)),
	("Yellow", BitSet::from_iter(97+8+8..97+8+8+8)),
	("Rebirth", BitSet::from_iter(0..10)),
	("Killer Inside", BitSet::from_iter(10..20)),
	("Only Shallow", BitSet::from_iter(20..30)),
	("The Old City", BitSet::from_iter(30..33)),
	("The Burn That Cures", BitSet::from_iter(33..43)),
	("Covenant", BitSet::from_iter(43..53)),
	("Reckoning", BitSet::from_iter(53..63)),
	("Benediction", BitSet::from_iter(63..73)),
	("Apocrypha", BitSet::from_iter(73..83)),
	("The Third Temple", BitSet::from_iter(83..85)),
	("Thousand Pound Butterfly", BitSet::from_iter(85..95)),
	("Hand of God", BitSet::from_iter(95..97)),
	("Rainbow", BitSet::from_iter(97..97+8+8+8)),
	("Boss", BitSet::from_iter([32, 82, 96])),
]));

#[allow(unused)]
struct DotnetRandom {
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

#[allow(unused)]
const MSEED: i32 = 161803398;

impl DotnetRandom {
	#![allow(unused)]
	fn with_seed(seed: i32) -> Self {
		let mut res = Self::default();
		res.set_seed(seed);
		res
	}

	fn set_seed(&mut self, mut seed: i32) {
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

	fn next(&mut self) -> i32 {
		let i1 = if self.i + 21 >= 55 { self.i + 21 - 55 } else { self.i + 21 } + 1;
		self.i = if self.i == 55 { 1 } else { self.i + 1 };
		let mut num3 = self.state[self.i] - self.state[i1];
		if num3 < 0 { num3 += i32::MAX };
		self.state[self.i] = num3;
		num3
	}

	fn next_biased_range(&mut self, min: i32, max: i32) -> i32 {
		(self.next_double()*(max - min)as f64)as i32+min
	}

	fn next_double(&mut self) -> f64 {
		self.next()as f64/i32::MAX as f64
	}

	fn get_shuffled_idxs(num_levels: usize, seed: i32, out: &mut Vec<u8>) {
		out.clear();
		(0..num_levels as u8).collect_into(out);
		let mut random = DotnetRandom::with_seed(seed);
		for i in 0..num_levels {
			out.swap(i, random.next_biased_range(0, num_levels as _)as _);
		}
	}
}


fn get_shuffled_idxs(num_levels: u8, seed: i32, out: &mut Vec<u8>) {
	out.clear();
	(0..num_levels).collect_into(out);
	const AS: [i32; 96] = [1121899819, 630111683, 1501065279, 458365203, 969558243, 1876681249, 962194431, 1077359051, 265679591, 791886952, 1582116761, 1676571504, 1476289907, 1117239683, 1503178135, 1341148412, 902714229, 1331438416, 58133212, 831516153, 285337308, 526856546, 362935496, 750214563, 210465667, 1381224997, 1846331200, 1330597961, 593162892, 1729496551, 792803163, 565661843, 863554642, 53838754, 749855384, 93067682, 1778866589, 1463507567, 367760674, 1219347826, 1648614489, 596622148, 1228675679, 243017841, 1132230640, 1891159862, 730619752, 33642253, 209795643, 283831563, 249493290, 967871855, 1560699908, 437500212, 429989927, 595043273, 267176187, 750850716, 247899536, 1735816893, 30350049, 1779080117, 484196159, 683666687, 2146567436, 1016454918, 813016862, 1422451153, 367384299, 1410110453, 1709765470, 1586690309, 963677742, 986269033, 1330385311, 1836198807, 1445664514, 119917655, 1765467570, 466789452, 650605245, 1812688947, 1120802318, 309331329, 1480003261, 1972414955, 1152445582, 426054430, 1771332474, 154812111, 1973375142, 1028015873, 1215608031, 779427428, 1188997777, 2017018019];
	const BS: [i32; 96] = [1559595546, 1755192844, 1649316166, 1198642031, 442452829, 1200195957, 1945678308, 949569752, 2099272109, 587775847, 626863973, 1003550677, 1358625013, 1008269081, 2109153755, 65212616, 1851925803, 2137491580, 1454235444, 675580731, 1754296375, 1821177336, 2130093701, 70062080, 1503113964, 1130186590, 2005789796, 1476653312, 1174277203, 174182291, 401846963, 973512717, 638171722, 2122881600, 1380182313, 1638451829, 65271247, 818200948, 736891500, 2056119311, 1084756724, 1537539262, 255459778, 587232589, 1947978014, 1706746116, 724046315, 981848395, 315304373, 475269784, 880625662, 1543454120, 1331075398, 1047903413, 418573418, 1885901857, 1772582790, 1579254086, 1843011714, 1459749886, 1341889808, 469024996, 1922776196, 1925089818, 185928884, 1800834903, 365378955, 1383227060, 1775570415, 470701926, 2147425016, 1033724855, 1400600080, 1545599780, 1738307654, 216757113, 1565717558, 1542861112, 269567713, 1943851495, 406140275, 1023941401, 1161348939, 699007419, 1441040276, 1005876490, 1789920966, 1737751956, 1704308182, 1641764103, 2013352686, 633500808, 1122672881, 1424625261, 714229503, 615731728];
	for (i, (&a, &b)) in AS.iter().zip(&BS).take(num_levels as _).enumerate() {
		let numer = (((a as u64)*seed as u64 + (b as u64))%i32::MAX as u64)as f64;
		let j = (numer/i32::MAX as f64*num_levels as f64)as i32;
		out.swap(i, j as _)
	}
}

fn is_abbreviation(cleanedname: &str, fullname: &str) -> bool {
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

fn lookup_name(name: &str, level_set: &BitSet) -> AResult<usize> {
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

enum Rule {
	Sequence(usize, usize, Vec<u8>),
	Subset(usize, usize, BitSet)
}

impl Rule {
	fn len(&self) -> usize {
		match self {
			Self::Sequence(_, _, v) => v.len(),
			Self::Subset(_, _, s) => s.len()
		}
	}

	fn iter_domain(&self) -> impl Iterator<Item=usize> {
		match self {
			Self::Sequence(a, b, _) | Self::Subset(a, b, _) => *a..*b
		}
	}

	fn iter_codomain(&self) -> Box<dyn Iterator<Item=usize> + '_> {
		match self {
			Self::Sequence(_, _, v) => Box::new(v.iter().copied().map_into()),
			Self::Subset(_, _, s) => Box::new(s.iter())
		}
	}

	fn matches(&self, buf: &[u8]) -> bool {
		match self {
			Self::Sequence(a, b, v) => buf[*a..*b].iter().zip(v).all(|(&a, &b)|a==b),
			Self::Subset(a, b, s) => BitSet::from_iter(buf[*a..*b].iter().copied().map_into()).is_superset(s)
		}
	}
}

fn print_info() {
	println!("neonwhite_seed_finder");
	println!("Written by hacatu, based on existing seed finders by");
	println!("Grange Nagy (https://github.com/Grange-Nagy/neonwhite_seed_generator)");
	println!("Static (https://github.com/stxticOVFL/RushSeedSearcher)");
	println!("");
	println!("This seed finder supports ");
}

fn print_usage() {
	println!("The usage should be:");
	println!(" (1) neonwhite_seed_finder [help]");
	println!(" (2) neonwhite_seed_finder [<Rush Abbreviation> [simulate] <seed>");
	println!(" (3) neonwhite_seed_finder [<Rush Abbreviation>] [find] [<max count>] <description>");
	println!(" (4) neonwhite_seed_finder [<Rush Abbreviation>] [best] [<max count>] <stats file>");
	println!("where");
	println!(" - <Rush Abbreviation> is");
	println!("    (W)hite/(M)ikey/(Red)/(Y)ellow/(V)iolet");
	println!("    /(Reb)irth/(K)iller Inside/Only (S)hallow/The Old (Ci)ty");
	println!("    /The Burn That (Cu)res/(Co)venant/(Rec)koning/(Be)nediction/(A)pocrypha/The Third (Te)mple");
	println!("    /Thousand (P)ound Butterfly/(H)and of God");
	println!("    /(Ra)inbow/(Bo)ss");
	println!("   This is case insensitive, and an abbreviation like \"w\" can be used if it is unambiguous.");
	println!("   The abbreviation matching algorithm is the same as for level names,");
	println!("   and numbers with 0=white/mikey, 1 through 12=chapter rushes, 13 through 15=sidequest rushes");
	println!("   can be used");
	println!(" - <seed> should be a number s with 0 <= s < 2^31");
	println!(" - <max count> should be a number c with 1 <= c <= 2^31,");
	println!("   and it should not be large if you expect many valid results");
	println!(" - <description>: the rest of the input is recombined into one string,");
	println!("   with the following format (whitespace outside of level names is ignored):");
	println!("    DESCRIPTION: RULE (\"&\" RULE)*");
	println!("    RULE: SEQUENCE_RULE | SUBSET_RULE");
	println!("    SEQUENCE_RULE: INDEX?\":\"INDEX? \"=\" ID_LIST");
	println!("    SUBSET_RULE: INDEX?\":\"INDEX? \">\" ID_LIST");
	println!("    INDEX: <an index in the python style>");
	println!("    ID_LIST: ID (\",\" ID)*");
	println!("    ID: INDEX | <abbreviated name>");
	println!("   For example, to make the first 3 levels be the boss fights in order, we could do");
	println!("   \"neonwhite_seed_finder white find 1 0:3 = The Clocktower, The Third Temple, Absolution\"");
	println!("   or even");
	println!("   \"neonwhite_seed_finder w f :3=clock,ttt,abso\"");
	println!("   If we instead want the last 3 levels to be godspeed, waterworks, and movement");
	println!("   in any order, we could do");
	println!("   \"neonwhite_seed_finder w f -3:>godsp,water,move\"");
	println!("   Or if we want the first 10 levels to include");
	println!("   escalation, bouquet, breakthrough, and closer, we could do");
	println!("   \"neonwhite_seed_finder w f :10>esc,bouq,break,clos\"");
	println!("   etc");
	println!(" - <stats file> contains info about your average and standard deviation on different levels");
	println!("The subcommand (simulate or find) may also be abbreviated");
	println!("The program will try to re-group the arguments to one of the forms (1-4) and then");
	println!("print this message, simulate the rush, or find a seed with the given discription");
	println!("\"best\" is TODO, it will find the best seeds that put your most inconsistent stages first");
}

fn guess_rush_from_abbr(rush_abbr: &[String]) -> AResult<&BitSet> {
	let cleanedname = rush_abbr.into_iter().map(|s|s.to_lowercase()).join(" ");
	if cleanedname.len() == 0 {
		return AOk(&LEVEL_SETS["White"]);
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
			return AOk(&LEVEL_SETS[KEYS[idx as usize]]);
		}
		if idx == 1000 {
			return AOk(&LEVEL_SETS["Thousand Pound Butterfly"]);
		}
	}
	let mut res: Option<(&str, &BitSet)> = None;
	for (&fullname, level_set) in LEVEL_SETS.iter() {
		if is_abbreviation(cleanedname.as_str(), fullname) {
			if res.is_some() {
				bail!("Rush abbreviation \"{cleanedname}\" is ambiguous and could match {} or {fullname}", res.unwrap().0);
			}
			res = Some((fullname, level_set));
		}
	}
	let Some((_, res)) = res else {
		bail!("Rush abbreviation \"{cleanedname}\" did not match anything");
	};
	AOk(res)
}

fn guess_rule_once(description: &str, level_set: &BitSet) -> AResult<Rule> {
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

fn guess_rules_from_description(description: &[String], level_set: &BitSet) -> AResult<Vec<Rule>> {
	let description = description.join(" ");
	let res: Vec<_> = description.split("&").map(|s|guess_rule_once(s, level_set)).try_collect()?;
	let mut domains = vec![None; level_set.len()];
	let mut codomains = vec![None; level_set.len()];
	for (i, rule) in res.iter().enumerate() {
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
	AOk(res)
}

fn try_simulate(args: Vec<String>) -> AResult<()> {
	let (rush_abbr, seed) =
		if args.len() == 2 {
			(&args[1..1], &args[1])
		} else if let Some(s) = "help find best".split_whitespace().filter(|&s|s.starts_with(&args[args.len()-2])).next() {
			bail!("Bad subcommand format, looks like \"simulate\" but found \"{s}\"\nTry \"neonwhite_seed_finder help\" for details");
		} else if "simulate".starts_with(&args[args.len()-2]) {
			(&args[1..args.len()-2], &args[args.len()-1])
		} else {
			(&args[1..args.len()-1], &args[args.len()-1])
		};
	let level_set = guess_rush_from_abbr(rush_abbr)?;
	let seed = match i32::from_str_radix(seed.as_str(), 10) {
		Err(_) => bail!("Could not parse seed from \"{seed}\""),
		Ok(s) if s < 0 => bail!("Seed should not be negative"),
		Ok(s) => s
	};
	let mut buf = Vec::with_capacity(level_set.len());
	get_shuffled_idxs(level_set.len()as _, seed, &mut buf);
	print!("[");
	for i in buf {
		print!("{}, ", ALL_LEVELS[level_set.iter().nth(i as _).unwrap()])
	}
	println!("]");
	AOk(())
}

fn find_matching_seeds_cpu(level_count: usize, result_count: usize, rules: &[Rule]) -> AResult<impl Iterator<Item=i32>> {
	AOk((0..i32::MAX).into_par_iter().map_init(||Vec::with_capacity(level_count), |buf, s|{
		get_shuffled_idxs(level_count as _, s, buf);
		(s, rules.iter().all(|r|r.matches(buf)))
	}).filter_map(|(s, p)|if p {Some(s)} else {None}).take_any(result_count).collect_vec_list().into_iter().flatten())
}

fn try_find(args: Vec<String>) -> AResult<()> {
	let idx = args.iter().skip(1).take_while(|&s|!s.contains(":")).count() + 1;
	let have_count = idx > 1 && i32::from_str_radix(&args[idx-1], 10).is_ok();
	let have_subcommand = if idx > 1 {
		let i = if have_count { idx - 2 } else { idx - 1 };
		if let Some(s) = "help simulate best".split_whitespace().filter(|&s|s.starts_with(&args[i])).next() {
			bail!("Bad subcommand format, looks like \"find\" but found \"{s}\"\nTry \"neonwhite_seed_finder help\" for details");
		}
		"find".starts_with(&args[i])
	} else { false };
	let (rush_abbr, count, description) = (
		&args[1..idx-have_count as usize-have_subcommand as usize],
		if have_count {i32::from_str_radix(&args[idx-1], 10).unwrap()} else {1},
		&args[idx..]
	);
	let level_set = guess_rush_from_abbr(rush_abbr)?;
	let rules = guess_rules_from_description(description, level_set)?;
	let it = find_matching_seeds_cpu(level_set.len(), count as _, &rules)?;
	print!("[");
	for s in it {
		print!("{s},");
	}
	println!("]");
	AOk(())
}

fn try_best(args: Vec<String>) -> AResult<()> {
	todo!()
}

fn main() -> AResult<()> {
	let args = std::env::args().collect_vec();
	if args.iter().any(|s|s.contains(":")) {
		try_find(args)
	} else if args.iter().any(|s|s.contains(".")) {
		try_best(args)
	} else if args.len() == 1 || (args.len() == 2 && "help".starts_with(&args[1])) {
		print_info();
		print_usage();
		AOk(())
	} else {
		try_simulate(args)
	}
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::{DotnetRandom, LEVEL_SETS, get_shuffled_idxs, lookup_name};

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
	use crate::{DotnetRandom, get_shuffled_idxs};

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

