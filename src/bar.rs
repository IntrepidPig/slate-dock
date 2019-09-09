pub struct BarConfig {
	pub height: u32,
	pub background_color: (f64, f64, f64),
	pub font_color: (f64, f64, f64),
	pub font_size: f64,
}

impl Default for BarConfig {
	fn default() -> Self {
		BarConfig {
			height: 48,
			background_color: (0.192, 0.2, 0.219),
			font_color: (0.92, 0.92, 0.92),
			font_size: 18.0,
		}
	}
}
