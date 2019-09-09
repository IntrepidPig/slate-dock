use std::time::{Duration, Instant};

use crate::window::Rectangle;
pub use xcb;

use raw_brass::event::{MouseButton, PressState};
use raw_brass::window::xcb::config::ConfigValue;
use raw_brass::window::xcb::property::{AtomProperty, CardinalProperty};
use raw_brass::window::WindowEvent::MouseClick;
use raw_brass::{
	app::App,
	drawing::{
		cairo::{CairoBackend, CairoSurface},
		DrawingBackend, SurfaceCreator,
	},
	window::{
		xcb::{XcbBackend, XcbBackendError, XcbWindow},
		WindowBackend, WindowDims, WindowEvent,
	},
};
use std::collections::VecDeque;

pub mod bar;
pub mod window;

fn main() {
	init_logging();

	let bar_config = bar::BarConfig::default();

	let (conn, screen_idx) = window::init_xcb()
		.map_err(|e| {
			log::error!("Failed to connect to X Server: {}", e);
			std::process::exit(1);
		})
		.unwrap();
	let screen = window::get_screen(&conn, screen_idx);

	let primary_monitor_dims = window::get_primary_monitor_dims(&conn, &screen)
		.map_err(|e| {
			log::error!("Failed to get the dimensions of the primary monitor: {}", e);
			std::process::exit(1);
		})
		.unwrap();

	let bar_dims = Rectangle {
		x: primary_monitor_dims.x,
		y: primary_monitor_dims.y + primary_monitor_dims.height as i32 - bar_config.height as i32,
		width: primary_monitor_dims.width,
		height: bar_config.height,
	};

	log::info!("Creating bar with dimensions {:?}", bar_dims);

	let window_backend = XcbBackend::init().unwrap();
	let window = window_backend
		.create_window(WindowDims {
			x: bar_dims.x,
			y: bar_dims.y,
			width: bar_dims.width,
			height: bar_dims.height,
		})
		.unwrap();

	window_backend
		.set_property::<_, AtomProperty>(
			window,
			window_backend.intern_atom("_NET_WM_WINDOW_TYPE").unwrap(),
			vec![AtomProperty(window_backend.intern_atom("_NET_WM_WINDOW_TYPE_DOCK").unwrap())],
		)
		.unwrap();

	window_backend
		.set_property::<_, CardinalProperty>(
			window,
			window_backend.intern_atom("_NET_WM_PARTIAL_STRUT").unwrap(),
			vec![
				CardinalProperty(0),
				CardinalProperty(0),
				CardinalProperty(0),
				CardinalProperty(bar_dims.height),
				CardinalProperty(0),
				CardinalProperty(primary_monitor_dims.height),
				CardinalProperty(0),
				CardinalProperty(primary_monitor_dims.height),
				CardinalProperty(0),
				CardinalProperty(primary_monitor_dims.width),
				CardinalProperty(0),
				CardinalProperty(primary_monitor_dims.width),
			],
		)
		.unwrap();

	window_backend
		.set_property::<_, CardinalProperty>(
			window,
			window_backend.intern_atom("_NET_WM_STRUT").unwrap(),
			vec![
				CardinalProperty(0),
				CardinalProperty(0),
				CardinalProperty(0),
				CardinalProperty(bar_dims.height),
			],
		)
		.unwrap();

	window_backend
		.configure_window(window, &[ConfigValue::BorderWidth(0)])
		.unwrap();
	window_backend.map_window(window).unwrap();

	let mut brass_window = raw_brass::window::xcb::XcbWindow { window };

	let surface = window_backend.create_surface(&brass_window);

	let start_x = 10.0;
	let end_x = 300.0;
	let mut t = 0.0;
	let mut dt = 0.016;

	let mut positions: Vec<(f64, f64, std::time::Instant)> = Vec::new();

	let mut draw_backend = CairoBackend::new(surface);
	draw_backend.ctx.set_font_size(bar_config.font_size);
	let font_extents = draw_backend.get_font_extents();

	let mut event_buf = VecDeque::new();
	loop {
		window_backend.get_window_events(&mut brass_window, &mut event_buf);
		while let Some(event) = event_buf.pop_front() {
			match event {
				WindowEvent::MouseClick(click) => {
					if click.state == PressState::Pressed {
						match click.button {
							MouseButton::Left => {
								println!("Got click at ({},{})", click.pos.0, click.pos.1);
								positions.push((click.pos.0, click.pos.1, Instant::now()));
							}
							_ => {}
						}
					}
				}
				WindowEvent::CloseHappened => {
					log::info!("Close requested");
					break;
				}
				_ => {}
			}
		}

		draw_backend.set_source_rgba(1.0, 1.0, 1.0, 0.0);
		draw_backend.clear();
		draw_backend.set_source_rgba(
			bar_config.background_color.0,
			bar_config.background_color.1,
			bar_config.background_color.2,
			0.975,
		);
		draw_backend.rect(0.0, 0.0, primary_monitor_dims.width as f64, bar_config.height as f64);
		draw_backend.fill();
		//draw_backend.set_source_rgba(bar_config.font_color.0, bar_config.font_color.1, bar_config.font_color.2, 1.0);
		//draw_backend.rect(coserp(start_x, end_x, t), 10.0, 50.0, 5.0);
		//draw_backend.fill();
		for pos in &positions {
			let elapsed = dur_to_secs(pos.2.elapsed());
			let alpha = lerp(1.0, 0.0, elapsed * (1.0 / 3.0));
			draw_backend.set_source_rgba(0.0, 1.0, 0.0, alpha);
			draw_backend.rect(pos.0 - 5.0, pos.1 - 5.0, 10.0, 10.0);
			draw_backend.fill();
		}
		draw_backend.set_source_rgba(bar_config.font_color.0, bar_config.font_color.1, bar_config.font_color.2, 1.0);
		let time = chrono::Local::now();
		let text: String = time.format("%A, %B/%d %-I:%M %p").to_string();
		let text_extents = draw_backend.get_text_extents(&text);
		draw_backend.move_to(
			1920.0 - ((bar_config.height as f64 - text_extents.height as f64) / 2.0) - text_extents.width,
			bar_config.height as f64 / 2.0 + text_extents.height / 2.0,
		);
		draw_backend.draw_text(&text);
		draw_backend.fill();
		positions = positions
			.into_iter()
			.filter(|pos| dur_to_secs(pos.2.elapsed()) < 3.0)
			.collect();
		draw_backend.present();

		t += dt;
		if t >= 1.0 {
			dt = -0.016;
		} else if t <= 0.0 {
			dt = 0.016;
		}

		std::thread::sleep_ms(16);
	}
}

fn coserp(v1: f64, v2: f64, t: f64) -> f64 {
	let x = t * std::f64::consts::PI;
	let curved_t = x.cos() * -0.5 + 0.5;

	let diff = v2 - v1;
	let d = diff * curved_t;
	v1 + d
}

fn lerp(v1: f64, v2: f64, t: f64) -> f64 {
	let diff = v2 - v1;
	let d = diff * t;
	v1 + d
}

fn dur_to_secs(dur: Duration) -> f64 {
	dur.as_secs() as f64 + (dur.subsec_nanos() as f64 / 1e9)
}

fn init_logging() {
	fern::Dispatch::new()
		.format(|out, message, record| out.finish(format_args!("[{}] {}", record.level(), message)))
		.level(log::LevelFilter::Trace)
		.chain(std::io::stderr())
		.apply()
		.unwrap();
}
