use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::text::Span;
use ratatui::widgets::Widget;

pub struct Checkbox<'s> {
	value: bool,
	style: Style,

	label: Span<'s>,
	markers: [Span<'s>; 2],
}

impl Checkbox<'_> {
	pub fn new(value: bool, label: Span) -> Self {
		Self {
			value,
			style: Style::default(),
			label,
			markers: [Span::from("[ ]"), Span::from("[x]")],
		}
	}

	pub fn style(mut self, style: Style) -> Self {
		self.style = style;
		self
	}

	pub fn markers(mut self, markers: [Span; 2]) -> Self {
		self.markers = markers;
		self
	}

	pub fn value(&self) -> bool {
		self.value
	}

	pub fn toggle(&mut self) {
		self.value = !self.value;
	}
}

impl Widget for &Checkbox<'_> {
	fn render(&self, mut area: Rect, buf: &mut Buffer)
	where
		Self: Sized,
	{
		let width = self.markers[self.value as usize].width();

		(&self.markers[self.value as usize]).render(area, buf);
		area.x += width + 1;
		area.width -= width + 1;
		(&self.label).render(area, buf);
	}
}
