use clap::Parser;
use eframe::{egui::ViewportBuilder, epaint::Vec2, NativeOptions};
use figment::{
	providers::{Env, Format, Serialized, Toml},
	Figment,
};
use task_timer::{Application, Settings};
use tokio::runtime::Runtime;

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	let config: task_timer::Settings = Figment::new()
		.merge(Serialized::defaults(Settings::parse()))
		.merge(Toml::file("task-timer.toml"))
		.merge(Env::prefixed("TASK_TIMER_"))
		.extract()?;

	let mut app = Application::init(Runtime::new()?, config);
	app.runtime
		.block_on(async { app.events.reset(&app.settings.calendar) });

	eframe::run_native(
		"Task Timer",
		NativeOptions {
			viewport: ViewportBuilder::default()
				.with_always_on_top()
				// .with_decorations(false)
				.with_inner_size(Vec2::new(300.0, 150.0)),
			..Default::default()
		},
		Box::new(move |_| Box::new(app)),
	)
	.expect("App crash");

	Ok(())
}
