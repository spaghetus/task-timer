use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, Offset, Utc};
use chumsky::error::Simple;
use clap::Args;
use minicaldav::Credentials;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::task::JoinHandle;
use url::Url;

#[derive(Args, Debug, Serialize, Deserialize, Clone)]
pub struct CalendarSettings {
	#[arg(long = "calendar", short = 'c')]
	pub urls: Vec<Url>,
	#[arg(long, short)]
	pub username: Option<String>,
	#[arg(long, short)]
	pub password: Option<String>,
	#[arg(long, short)]
	pub token: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Event {
	pub uid: String,
	pub date_stamp: DateTime<Local>,
	pub summary: String,
	pub starts: Option<DateTime<Local>>,
	pub due: Option<DateTime<Local>>,
	pub priority: i8,
}

pub enum Calendar {
	Working(Option<JoinHandle<Vec<Event>>>),
	Ready(Vec<Event>),
}

impl Default for Calendar {
	fn default() -> Self {
		Calendar::Ready(vec![])
	}
}

fn parse_ical_date(input: &str) -> Result<DateTime<Local>, Vec<Simple<char>>> {
	use chumsky::prelude::*;
	fn number(length: usize) -> impl Parser<char, u32, Error = Simple<char>> {
		one_of("1234567890")
			.map(|n: char| -> u32 { n.to_digit(10).unwrap() })
			.repeated()
			.exactly(length)
			.map(|digits| digits.into_iter().fold(0, |acc, el| (acc * 10) + el))
	}
	fn date() -> impl Parser<char, NaiveDate, Error = Simple<char>> {
		number(4)
			.then(number(2))
			.then(number(2))
			.map(|((y, m), d)| NaiveDate::from_ymd_opt(y as i32, m, d).unwrap())
	}
	fn time() -> impl Parser<char, NaiveTime, Error = Simple<char>> {
		number(2)
			.then(number(2))
			.then(number(2))
			.map(|((h, m), s)| NaiveTime::from_hms_opt(h, m, s).unwrap())
	}
	fn datetime() -> impl Parser<char, DateTime<Local>, Error = Simple<char>> {
		date()
			.then(
				just('T')
					.ignore_then(time())
					.then(just('Z').ignored().or_not())
					.or_not(),
			)
			.map(|(date, time)| match time {
				None => DateTime::<Utc>::from_naive_utc_and_offset(
					NaiveDateTime::new(date, NaiveTime::default()),
					Utc,
				)
				.into(),
				Some((time, Some(_))) => {
					DateTime::<Utc>::from_naive_utc_and_offset(NaiveDateTime::new(date, time), Utc)
						.into()
				}
				Some((time, None)) => DateTime::<Local>::from_naive_utc_and_offset(
					NaiveDateTime::new(date, time),
					Utc.fix(),
				),
			})
	}

	datetime().parse(input)
}

impl Calendar {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn reset(&mut self, settings: &CalendarSettings) {
		let settings = settings.clone();
		*self = Self::Working(Some(tokio::task::spawn_blocking(move || {
			let urls = settings.urls.clone();
			let agent = ureq::agent();
			let credentials = match settings {
				CalendarSettings {
					username: Some(username),
					password: Some(password),
					..
				} => Credentials::Basic(username, password),
				CalendarSettings {
					token: Some(token), ..
				} => Credentials::Bearer(token),
				_ => Credentials::Bearer(String::new()),
			};

			urls.into_iter()
				.flat_map(|url| minicaldav::get_calendars(agent.clone(), &credentials, &url))
				.flat_map(|calendars| calendars.into_iter())
				.flat_map(|calendar| minicaldav::get_todos(agent.clone(), &credentials, &calendar))
				.flat_map(|(events, _errors)| events.into_iter())
				.map(|event| {
					event
						.properties_todo()
						.into_iter()
						.map(|(k, v)| (k.to_lowercase(), v.to_string()))
						.collect::<HashMap<_, _>>()
				})
				.filter(|event| event.get("status").map(|s| s.as_str()) != Some("COMPLETED"))
				.filter(|event| event.get("completed").is_none())
				.filter(|event| event.get("percent-complete").map(|s| s.as_str()) != Some("100"))
				.filter(|event| event.get("rrule").is_none())
				.map(|properties| Event {
					uid: properties
						.get("uid")
						.cloned()
						.unwrap_or_else(|| "???".to_string()),
					date_stamp: properties
						.get("dtstamp")
						.and_then(|dtstamp| parse_ical_date(dtstamp).ok())
						.unwrap_or_default(),
					summary: properties
						.get("summary")
						.cloned()
						.unwrap_or_else(|| "???".to_string()),
					starts: properties
						.get("dtstart")
						.and_then(|dtstart| parse_ical_date(dtstart).ok()),
					due: properties
						.get("due")
						.and_then(|due| parse_ical_date(due).ok()),
					priority: properties
						.get("priority")
						.and_then(|p| p.parse().ok())
						.unwrap_or(11i8),
				})
				.collect()
		})))
	}

	pub async fn tick(&mut self, _settings: &CalendarSettings) {
		match self {
			Calendar::Working(task) if task.is_some() && task.as_ref().unwrap().is_finished() => {
				*self = Self::Ready(
					std::mem::take(task)
						.unwrap()
						.await
						.expect("Calendar thread died somehow"),
				);
			}
			_ => {}
		}
	}
}
