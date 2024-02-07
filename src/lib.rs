use std::time::{Duration, Instant};

use calendar::{Calendar, Event};
use chrono::Local;
use clap::Parser;
use eframe::egui;
use pretty_duration::pretty_duration;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use timer::{Timer, TimerDiscriminants};
use tokio::runtime::Runtime;

pub mod calendar;
pub mod timer;

#[derive(Parser, Debug, Serialize, Deserialize)]
pub struct Settings {
	#[command(flatten)]
	pub timer: timer::TimerSettings,
	#[command(flatten)]
	pub calendar: calendar::CalendarSettings,
}

pub struct Application {
	pub settings: Settings,
	pub runtime: Runtime,
	pub timer: Timer,
	pub events: Calendar,
	pub chosen_event: Option<Event>,
	pub paused_for: Duration,
	pub paused_at: Option<Instant>,
}

impl Application {
	pub fn init(runtime: Runtime, settings: Settings) -> Self {
		Self {
			timer: Timer::default(),
			events: Calendar::default(),
			chosen_event: None,
			runtime,
			settings,
			paused_at: None,
			paused_for: Duration::ZERO,
		}
	}
	pub fn tick(&mut self, now: Instant) {
		if self.timer.tick(&self.settings.timer, now, false) {
			self.timer.ping();
		}
		self.runtime
			.block_on(self.events.tick(&self.settings.calendar));
		match (
			self.timer.working(),
			&self.events,
			self.chosen_event.is_some(),
		) {
			(_, Calendar::Ready(events), false) if !events.is_empty() => self.choose_event(),
			// (false, _, true) => self.chosen_event = None,
			_ => {}
		}
	}
	pub fn choose_event(&mut self) {
		let Calendar::Ready(events) = &self.events else {
			return;
		};
		let mut rng = thread_rng();
		let now = Local::now() - self.paused_for;
		let candidate_events: Vec<_> = events
			.iter()
			.filter(|event| {
				if let Some(start) = event.starts {
					start < now
				} else {
					true
				}
			})
			.flat_map(|event| {
				let mut priority = 12i16 - event.priority as i16;
				if let Some(due) = event.due {
					if due < now {
						priority = priority.saturating_mul(2).max(1);
					}
				}
				std::iter::repeat(event).take(priority as usize)
			})
			.collect();
		self.chosen_event = candidate_events.choose(&mut rng).map(|e| (*e).clone());
	}
}

impl eframe::App for Application {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let now = Instant::now()
			- self.paused_for
			- self
				.paused_at
				.map(|t| t.elapsed())
				.unwrap_or(Duration::ZERO);
		if self.paused_at.is_none() {
			self.tick(now);
			if self.timer.running() {
				ctx.request_repaint();
			}
		}
		egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
			ui.horizontal(|ui| {
				if matches!(self.events, Calendar::Working(_)) {
					ui.label("Calendar");
					ui.spinner();
				} else if ui.button("Reload").clicked() {
					self.runtime.block_on(async {
						self.events.reset(&self.settings.calendar);
					});
				}
				if self.timer.running() {
					if ui.button("Stop").clicked() {
						self.timer.stop();
					}
				} else if ui.button("Start").clicked() {
					self.timer.start(now);
				}
				if let Some(paused_at) = &self.paused_at {
					if ui.button("Resume").clicked() {
						self.paused_for += paused_at.elapsed();
						self.paused_at = None;
					}
				} else if ui.button("Pause").clicked() {
					self.paused_at = Some(Instant::now());
				}
				if ui.button("Skip").clicked() {
					self.timer.tick(&self.settings.timer, now, true);
					self.timer.ping();
				}
			});
		});
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				let duration =
					pretty_duration(&self.timer.remaining(now, &self.settings.timer), None);
				let phase: TimerDiscriminants = self.timer.into();
				ui.heading(format!("{phase:?} : {duration}"));
				if let Some(event) = &self.chosen_event {
					ui.label(format!("E: {}", event.summary));
					if let Some(start) = &event.starts {
						ui.label(format!("S: {start}"));
					}
					if let Some(due) = &event.due {
						ui.label(format!("D: {due}"));
					}
					if ui.button("New task").clicked() {
						self.choose_event();
					}
				}
			})
		});
	}
}
