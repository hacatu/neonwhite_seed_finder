#![feature(iter_collect_into, test)]
mod backend;
mod backend_opencl;
mod test_helpers;
mod data;
mod frontend;
#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "cli")]
mod cli;

use itertools::Itertools;
use anyhow::{Result as AResult, Ok as AOk, bail};
#[cfg(feature = "gui")]
use crate::gui::EguiApp;
#[cfg(feature = "cli")]
use crate::cli::{print_info, print_usage, try_find, try_simulate, try_best};

fn main() -> AResult<()> {
	#[cfg(feature = "cli")] {
		let args = std::env::args().collect_vec();
		if args.iter().skip(1).any(|s|s.contains(":")) {
			try_find(args)
		} else if args.iter().skip(1).any(|s|s.contains(".")) {
			try_best(args)
		} else if args.len() == 2 && "help".starts_with(&args[1]) {
			print_info();
			print_usage();
			AOk(())
		} else if args.len() == 1 {
			#[cfg(feature = "gui")] {
				let options = eframe::NativeOptions::default();
				if let eframe::Result::Err(e) = eframe::run_native("Neonwhite Seed Finder", options, Box::new(|cc|Ok(Box::new(EguiApp::new(cc))))) {
					bail!("{e}");
				}
				AOk(())
			}
			#[cfg(not(feature = "gui"))] {
				print_info();
				print_usage();
				AOk(())
			}
		} else {
			try_simulate(args)
		}
	}
	#[cfg(all(feature = "gui", not(feature = "cli")))]
	{
		let options = eframe::NativeOptions::default();
		if let eframe::Result::Err(e) = eframe::run_native("Neonwhite Seed Finder", options, Box::new(|cc|Ok(Box::new(EguiApp::new(cc))))) {
			bail!("{e}");
		}
		AOk(())
	}
}

