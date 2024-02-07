use clap::Args;
use notify_rust::Notification;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use std::time::Instant;
use strum::EnumDiscriminants;

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct TimerSettings {
	#[arg(long = "work-time", default_value = "1500")]
	pub work_time: f64,
	#[arg(long = "short-rest-time", default_value = "600")]
	pub short_rest_time: f64,
	#[arg(long = "long-rest-time", default_value = "1800")]
	pub long_rest_time: f64,
	#[arg(long = "long-rest-interval", default_value = "4")]
	pub long_rest_interval: u8,
}

#[derive(Default, Clone, Copy, EnumDiscriminants)]
pub enum Timer {
	#[default]
	NotRunning,
	Working(u8, Instant),
	ShortBreak(u8, Instant),
	LongBreak(Instant),
}

impl Timer {
	pub fn start(&mut self, now: Instant) {
		*self = Timer::Working(0, now)
	}
	pub fn tick(&mut self, settings: &TimerSettings, now: Instant, skip: bool) -> bool {
		match &self {
			Timer::Working(since_last_lbreak, started_at)
				if skip || (now - *started_at).as_secs_f64() >= settings.work_time =>
			{
				let since_last_lbreak = since_last_lbreak + 1;
				*self = if since_last_lbreak >= settings.long_rest_interval {
					Timer::LongBreak(now)
				} else {
					Timer::ShortBreak(since_last_lbreak, now)
				}
			}
			Timer::ShortBreak(since_last_lbreak, started_at)
				if skip || (now - *started_at).as_secs_f64() >= settings.short_rest_time =>
			{
				*self = Timer::Working(*since_last_lbreak, now)
			}
			Timer::LongBreak(started_at)
				if skip || (now - *started_at).as_secs_f64() >= settings.long_rest_time =>
			{
				*self = Timer::Working(0, now)
			}
			_ => return false,
		}
		true
	}
	pub fn working(&self) -> bool {
		matches!(self, Timer::Working(_, _))
	}
	pub fn running(&self) -> bool {
		!matches!(self, Timer::NotRunning)
	}
	pub fn stop(&mut self) {
		*self = Timer::NotRunning;
	}
	pub fn remaining(&self, now: Instant, settings: &TimerSettings) -> Duration {
		match self {
			Timer::NotRunning => Duration::ZERO,
			Timer::Working(_, started) => {
				(*started + Duration::from_secs_f64(settings.work_time)) - now
			}
			Timer::ShortBreak(_, started) => {
				(*started + Duration::from_secs_f64(settings.short_rest_time)) - now
			}
			Timer::LongBreak(started) => {
				(*started + Duration::from_secs_f64(settings.long_rest_time)) - now
			}
		}
	}
	pub fn ping(&self) {
		if let Err(e) = Notification::new()
			.summary("Pomodoro timer")
			.body(&format!("{:?}", TimerDiscriminants::from(self)))
			.show()
		{
			eprintln!("{e}");
		}
	}
}

#[test]
pub fn correct_breaks() {
	use TimerDiscriminants::*;
	let settings = TimerSettings {
		work_time: 2.0,
		short_rest_time: 1.0,
		long_rest_time: 3.0,
		long_rest_interval: 3,
	};
	let mut now = Instant::now();
	let mut states = vec![];
	let mut timer = Timer::default();
	timer.start(now);

	for i in 0..32 {
		states.push(TimerDiscriminants::from(&timer));
		now += Duration::from_secs(1);
		timer.tick(&settings, now, false);
	}

	assert_eq!(
		states,
		[
			Working, Working, ShortBreak, Working, Working, ShortBreak, Working, Working,
			LongBreak, LongBreak, LongBreak, Working, Working, ShortBreak, Working, Working,
			ShortBreak, Working, Working, LongBreak, LongBreak, LongBreak, Working, Working,
			ShortBreak, Working, Working, ShortBreak, Working, Working, LongBreak, LongBreak
		]
	)
}
