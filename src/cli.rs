#![cfg(feature = "cli")]
use anyhow::{Result as AResult, Ok as AOk, bail};
use crate::{
	backend::{estimate_result_count, find_matching_seeds_cpu, get_shuffled_idxs},
	backend_opencl::{find_matching_seeds_gpu, try_setup_gpu},
	data::ALL_LEVELS,
	frontend::{guess_rules_from_description, guess_rush_from_abbr}
};

pub fn print_info() {
	println!("neonwhite_seed_finder");
	println!("Written by hacatu, based on existing seed finders by");
	println!("Grange Nagy (https://github.com/Grange-Nagy/neonwhite_seed_generator)");
	println!("Static (https://github.com/stxticOVFL/RushSeedSearcher)");
	println!("");
	println!("This seed finder supports ");
}

pub fn print_usage() {
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

pub fn try_simulate(args: Vec<String>) -> AResult<()> {
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
	println!("INFO: Simulating seed {seed}");
	let (rush_name, level_set) = guess_rush_from_abbr(rush_abbr)?;
	println!("INFO: Using rush name \"{rush_name}\"");
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

pub fn try_find(args: Vec<String>) -> AResult<()> {
	let idx = args.iter().skip(1).take_while(|&s|!s.contains(":")).count() + 1;
	let have_count = idx > 1 && i32::from_str_radix(&args[idx-1], 10).is_ok();
	let have_subcommand = if idx > 1 {
		let i = if have_count { idx - 2 } else { idx - 1 };
		if let Some(s) = "help simulate best".split_whitespace().filter(|&s|s.starts_with(&args[i])).next() {
			bail!("Bad subcommand format, looks like \"find\" but found \"{s}\"\nTry \"neonwhite_seed_finder help\" for details");
		}
		"find".starts_with(&args[i])
	} else { false };
	println!("INFO: Finding seeds");
	let (rush_abbr, count, description) = (
		&args[1..idx-have_count as usize-have_subcommand as usize],
		if have_count {i32::from_str_radix(&args[idx-1], 10).unwrap()} else {1},
		&args[idx..]
	);
	if count < 1 {
		bail!("Count {count} is too small, it should be at least 1");
	}
	let (rush_name, level_set) = guess_rush_from_abbr(rush_abbr)?;
	println!("INFO: Using rush name \"{rush_name}\", keeping up to {count} result(s)");
	let rules = guess_rules_from_description(description, level_set)?;
	let estimated_result_count = estimate_result_count(level_set.len(), &rules);
	let want_gpu = estimated_result_count as i32/count < 100;
	println!("INFO: estimated result count: {estimated_result_count}; {}", if want_gpu { "trying to use GPU" } else { "using CPU" });
	if want_gpu && let Some(gpu) = try_setup_gpu()? {
		// TODO: dropping the gpu buffers immediately is fine for a one-shot cli, but in a batch mode cli or gui
		// we should cache it
		let it = find_matching_seeds_gpu(level_set.len(), count as _, &rules, &gpu)?;
		print!("[");
		for s in it { print!("{s},"); }
		println!("]");
		return AOk(());
	}
	if want_gpu {
		println!("WARNING: No GPU found but estimated result count is small.  A full search of all 2^31 seeds could take a few minutes.");
	}
	let it = find_matching_seeds_cpu(level_set.len(), count as _, &rules)?;
	print!("[");
	for s in it { print!("{s},"); }
	println!("]");
	AOk(())
}

pub fn try_best(_args: Vec<String>) -> AResult<()> {
	todo!("Finding best seeds for reset efficiency is not yet implemented")
}

