use std::fmt;

pub fn init_xcb<'a>() -> Result<(xcb::Connection, i32), XcbError> {
	let (conn, screen_idx) = xcb::Connection::connect(None).map_err(|e| XcbError)?;
	Ok((conn, screen_idx))
}

pub fn get_screen(conn: &xcb::Connection, screen_idx: i32) -> xcb::Screen {
	let setup = conn.get_setup();
	let mut screen = setup.roots().nth(screen_idx as usize).unwrap();
	screen
}

pub fn get_primary_monitor_dims(conn: &xcb::Connection, screen: &xcb::Screen) -> Result<Rectangle, XcbError> {
	let root = screen.root();
	let randr_screen_resources = xcb::randr::get_screen_resources_current(conn, root)
		.get_reply()
		.map_err(|_| XcbError)?;
	let output_infos = randr_screen_resources
		.outputs()
		.iter()
		.map(|output_id| {
			xcb::randr::get_output_info(conn, *output_id, randr_screen_resources.config_timestamp())
				.get_reply()
				.map_err(|_| XcbError)
		})
		.collect::<Result<Vec<_>, _>>()?;
	log::trace!(
		"All outputs: {:?}",
		output_infos
			.iter()
			.map(|info| String::from_utf8_lossy(info.name()))
			.collect::<Vec<_>>()
	);
	let primary_output = xcb::randr::get_output_primary(conn, root).get_reply().map_err(|_| XcbError)?;
	let primary_output_info = xcb::randr::get_output_info(conn, primary_output.output(), xcb::CURRENT_TIME)
		.get_reply()
		.map_err(|_| XcbError)?;
	let primary_output_name = String::from_utf8_lossy(primary_output_info.name());
	log::info!("Primary Output: {}", primary_output_name);
	let primary_crtc = primary_output_info.crtc();
	let primary_crtc_info = xcb::randr::get_crtc_info(conn, primary_crtc, primary_output_info.timestamp())
		.get_reply()
		.map_err(|_| XcbError)?;
	let rect = Rectangle {
		x: primary_crtc_info.x() as i32,
		y: primary_crtc_info.y() as i32,
		width: primary_crtc_info.width() as u32,
		height: primary_crtc_info.height() as u32,
	};
	log::info!(
		"Got primary monitor dimensions {}x{} at ({}, {}) from output {}",
		rect.width,
		rect.height,
		rect.x,
		rect.y,
		primary_output_name,
	);
	Ok(rect)
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
	pub x: i32,
	pub y: i32,
	pub width: u32,
	pub height: u32,
}

pub struct XcbError;

impl fmt::Display for XcbError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "An error occurred with XCB")
	}
}
