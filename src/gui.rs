#![cfg(feature = "gui")]
use std::{sync::Arc, thread::{JoinHandle, spawn}, time::Instant};
use itertools::Itertools;
use bit_set::BitSet;
use anyhow::{Result as AResult, Ok as AOk, bail};
use eframe::egui;
use crate::{
	backend::{get_shuffled_idxs, estimate_result_count, find_matching_seeds_cpu},
	backend_opencl::{Gpu, find_matching_seeds_gpu, try_setup_gpu},
	data::{ALL_LEVELS, LEVEL_SETS},
	frontend::{guess_rush_from_abbr, guess_rule_once, check_rules},
};

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
enum AppTab { #[default] Find, Simulate }

pub struct EguiApp {
	tab: AppTab,
	rush_type: String,
	level_set: Option<&'static BitSet>,
	result_count: String,
	rule_descriptions: Vec<String>,
	looked_for_gpu: bool,
	gpu: Arc<Option<Gpu>>,
	find_inputs_changed: bool,
	find_thread: Option<JoinHandle<AResult<String>>>,
	start_time: Instant,
	find_results: String,
	sim_seed: String,
	sim_results: Vec<String>,
	status: String,
	error_dialog: Option<String>
}

impl Default for EguiApp {
	fn default() -> Self {
		Self {
			tab: AppTab::Find,
			rush_type: "White".to_string(),
			level_set: LEVEL_SETS.get(&"White"),
			result_count: "1".to_string(),
			rule_descriptions: Vec::new(),
			looked_for_gpu: false,
			gpu: Arc::new(None),
			find_inputs_changed: false,
			find_thread: None,
			start_time: Instant::now(),
			find_results: String::new(),
			sim_seed: "0".to_string(),
			sim_results: Vec::with_capacity(96),
			status: "Ready".to_string(),
			error_dialog: None
		}
	}
}

fn update_status<T>(status: &mut String, result: AResult<T>) {
	match result {
		AResult::Err(e) => *status = format!("WARNING: {e}"),
		AResult::Ok(_) => *status = "Ready".to_string()
	}
}

impl EguiApp {
	pub fn new(_cc: &eframe::CreationContext) -> Self {
		Self::default()
	}

	fn update_simulate(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui|{
			ui.horizontal(|ui|{
				if ui.button("Find").clicked() {
					self.tab = AppTab::Find;
				}
				ui.label("Simulate")
			});
			ui.horizontal(|ui|{
				ui.label("Rush Type:");
				if ui.text_edit_singleline(&mut self.rush_type).lost_focus() {
					let t = self.parse_rush_abbr();
					update_status(&mut self.status, t);
				}
			});
			ui.horizontal(|ui|{
				ui.label("Seed:");
				if ui.text_edit_singleline(&mut self.sim_seed).lost_focus() {
					let t = self.parse_sim_seed();
					update_status(&mut self.status, t);
				}
			});
			if ui.button("Simulate").clicked() {
				if let AResult::Err(e) = self.run_simulate() {
					self.error_dialog = Some(e.to_string());
				}
			}
			ui.separator();
			ui.label("Results:");
			egui::ScrollArea::vertical().max_height(600.0).show(ui, |ui|{
				for (i, level) in self.sim_results.iter().enumerate() {
					ui.label(format!("{}: {level}", i+1));
				}
			});
			ui.separator();
			ui.label(&self.status);
		});
		if self.find_thread.is_some() {
			if let AResult::Err(e) = self.check_find() {
				self.error_dialog = Some(e.to_string());
			}
		}
		if self.error_dialog.is_some() {
			egui::Window::new("Error").collapsible(false).show(ctx, |ui|{
				ui.label(self.error_dialog.as_ref().unwrap());
				if ui.button("Ok").clicked() {
					self.error_dialog = None;
				}
			});
		}
	}

	fn parse_rush_abbr(&mut self) -> AResult<()> {
		self.find_inputs_changed = true;
		let (rush_name, level_set) = guess_rush_from_abbr(std::slice::from_ref(&self.rush_type))?;
		self.rush_type = rush_name.to_string();
		self.level_set = Some(level_set);
		AOk(())
	}

	fn parse_sim_seed(&mut self) -> AResult<i32> {
		match i32::from_str_radix(self.sim_seed.as_str(), 10) {
			Err(_) => bail!("Could not parse seed from \"{}\"", self.sim_seed),
			Ok(s) if s < 0 => bail!("Seed should not be negative"),
			Ok(s) => AOk(s)
		}
	}

	fn parse_result_count(&mut self) -> AResult<usize> {
		self.find_inputs_changed = true;
		AOk(if self.result_count.trim().len() == 0 {
			1
		} else {
			let count = usize::from_str_radix(self.result_count.trim(), 10)?;
			if count < 1 {
				bail!("Count {count} is too small, it should be at least 1");
			}
			count
		})
	}

	fn run_simulate(&mut self) -> AResult<()> {
		self.find_results.clear();
		self.parse_rush_abbr()?;
		let level_set = self.level_set.unwrap(); // parse_rush_abbr will return Err if this is not set
		let seed = self.parse_sim_seed()?;
		let mut buf = Vec::with_capacity(level_set.len());
		get_shuffled_idxs(level_set.len()as _, seed, &mut buf);
		buf.into_iter().map(|i|ALL_LEVELS[level_set.iter().nth(i as _).unwrap()].to_string()).collect_into(&mut self.sim_results);
		AOk(())
	}

	fn update_find(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui|{
			ui.horizontal(|ui|{
				ui.label("Find");
				if ui.button("Simulate").clicked() {
					self.tab = AppTab::Simulate;
				}
			});
			ui.horizontal(|ui|{
				ui.label("Rush Type:");
				if ui.text_edit_singleline(&mut self.rush_type).lost_focus() {
					let t = self.parse_rush_abbr();
					update_status(&mut self.status, t);
				}
			});
			ui.horizontal(|ui|{
				ui.label("Max Seeds to Find:");
				if ui.text_edit_singleline(&mut self.result_count).lost_focus() {
					let t = self.parse_result_count();
					update_status(&mut self.status, t);
				}
			});
			ui.horizontal(|ui|{
				if ui.button("Add Subset Rule").clicked() {
					self.rule_descriptions.push("0:3 > ttt".to_string());
					self.find_inputs_changed = true;
				}
				if ui.button("Add Sequence Rule").clicked() {
					self.rule_descriptions.push("0:1 = ttt".to_string());
					self.find_inputs_changed = true;
				}
			});
			egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui|{
				let mut to_remove = Vec::new();
				for (i, rule) in self.rule_descriptions.iter_mut().enumerate() {
					ui.horizontal(|ui|{
						if ui.text_edit_singleline(rule).lost_focus() {
							self.find_inputs_changed = true;
							if let Some(level_set) = self.level_set {
								update_status(&mut self.status, guess_rule_once(rule, level_set))
							} else {
								self.status = "WARNING: Can't check rule because \"Rush Type\" is invalid".to_string();
							}
						}
						if ui.button("(delete)").clicked() {
							to_remove.push(i);
						}
					});
				}
				if !to_remove.is_empty() {
					self.find_inputs_changed = true;
					let mut to_remove = to_remove.into_iter().peekable();
					let mut it = 0..;
					self.rule_descriptions.retain(|_|if it.next() == to_remove.peek().copied() { to_remove.next(); false } else { true });
				}
			});
			ui.horizontal(|ui|{
				if ui.button("Find!").clicked() {
					if let AResult::Err(e) = self.start_find() {
						self.error_dialog = Some(e.to_string());
					}
				}
				if self.find_thread.is_some() {
					if let AResult::Err(e) = self.check_find() {
						self.error_dialog = Some(e.to_string());
					}
				}
			});
			ui.separator();
			ui.label("Results:");
			egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui|{
				ui.label(&self.find_results);
			});
			ui.separator();
			ui.label(&self.status);
		});
		if self.error_dialog.is_some() {
			egui::Window::new("Error").collapsible(false).show(ctx, |ui|{
				ui.label(self.error_dialog.as_ref().unwrap());
				if ui.button("Ok").clicked() {
					self.error_dialog = None;
				}
			});
		}
	}

	fn start_find(&mut self) -> AResult<()> {
		if !self.find_inputs_changed {
			self.status = "INFO: Inputs have not changed since last search!".to_string();
			AOk(())
		} else if self.find_thread.is_some() {
			self.status = "INFO: A search is already in progress!".to_string();
			AOk(())
		} else {
			self.find_inputs_changed = false;
			self.find_results.clear();
			self.parse_rush_abbr()?;
			let level_set = self.level_set.unwrap();
			let result_count = self.parse_result_count()?;
			let rules: Vec<_> = self.rule_descriptions.iter().map(|s|guess_rule_once(s, level_set)).try_collect()?;
			check_rules(&rules, level_set)?;
			let estimated_result_count = estimate_result_count(level_set.len(), &rules);
			let want_gpu = estimated_result_count/result_count < 100;
			if want_gpu && !self.looked_for_gpu {
				self.looked_for_gpu = true;
				self.gpu = Arc::new(try_setup_gpu()?); // if it fails, self.gpu stays None
				// also notice that this REPLACES the Arc, so if the worker thread still had a copy,
				// it would be the only owner of the PREVIOUS version of the Arc, but obviously (to us)
				// that cannot happen
			}
			self.start_time = Instant::now();
			if want_gpu && self.gpu.is_some() {
				self.status = format!("INFO: estimated result count: {estimated_result_count}; using GPU \"{}\"", Option::as_ref(&self.gpu).unwrap().name());
				let gpu = self.gpu.clone();
				self.find_thread = Some(spawn(move||
					AOk(find_matching_seeds_gpu(level_set.len(), result_count as _, &rules, Option::as_ref(&gpu).unwrap())?.join(","))
				));
			} else {
				if want_gpu {
					self.status = format!("WARNING: estimated result count: {estimated_result_count}; could not find GPU so using CPU");
				} else {
					self.status = format!("INFO: estimated result count: {estimated_result_count}; using CPU");
				}
				self.find_thread = Some(spawn(move||
					AOk(find_matching_seeds_cpu(level_set.len(), result_count as _, &rules)?.join(","))
				));
			}
			println!("DEBUG: Launched worker thread!");
			AOk(())
		}
	}

	fn check_find(&mut self) -> AResult<()> {
		let Some(h) = self.find_thread.as_ref() else {
			return AOk(());
		};
		if !h.is_finished() {
			return AOk(());
		}
		let res = self.find_thread.take().unwrap().join();
		self.status = format!("INFO: Search finished in {:.3} seconds", self.start_time.elapsed().as_secs_f64());
		println!("DEBUG: Joined worker thread!");
		self.find_thread = None;
		match res {
			Ok(res) => self.find_results = res?,
			Err(_e) => bail!("Worker thread failed!")
		}
		AOk(())
	}
}

impl eframe::App for EguiApp {
	fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
		match self.tab {
			AppTab::Find => self.update_find(ctx, frame),
			AppTab::Simulate => self.update_simulate(ctx, frame)
		}
	}
}

