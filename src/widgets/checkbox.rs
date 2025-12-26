use std::sync::LazyLock;

use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Widget;
use ratatui::Frame;

use crate::widgets::widget::Component;

use super::widget::ComponentRenderCtx;

#[derive(Debug, Clone)]
pub struct CheckboxStyle<'s> {
	/// |<padding0>[x]<spacing>Label<padding1>|
	pub padding: [u16; 2],
	/// Spacing between marker and label
	pub spacing: u16,
	/// Checkbox markers
	pub markers: [Span<'s>; 2],
	/// Style override
	pub style: Option<Style>,
}

impl Default for CheckboxStyle<'_> {
	fn default() -> Self {
		Self {
			padding: Default::default(),
			spacing: 1,
			markers: ["[ ]".into(), "[x]".into()],
			style: Default::default(),
		}
	}
}

impl CheckboxStyle<'_> {
	pub fn style(&self) -> Style {
		match self.style {
			Some(style) => style.clone(),
			None => Style::default(),
		}
	}
}

static DEFAULT_STYLE: LazyLock<CheckboxStyle> = LazyLock::new(|| CheckboxStyle::default());

pub struct Checkbox<'s> {
	value: bool,
	style: &'s CheckboxStyle<'s>,

	label: Span<'s>,
}

impl Checkbox<'_> {
	pub fn new(value: bool, label: Span) -> Self {
		Self {
			value,
			style: &DEFAULT_STYLE,
			label,
		}
	}

	pub fn style(mut self, style: &CheckboxStyle) -> Self {
		self.style = style;
		self
	}

	pub fn value(&self) -> bool {
		self.value
	}

	pub fn toggle(&mut self) {
		self.value = !self.value;
	}
}

impl Component for Checkbox<'_> {
	fn input(&mut self, key: &KeyEvent) {
		if key.code == KeyCode::Char(' ') {
			self.toggle();
		}
	}

	fn render(&self, frame: &mut Frame, ctx: &ComponentRenderCtx) {
		let padding_left = Span::styled(
			" ".repeat(self.style.padding[0] as usize),
			self.style.style(),
		);
		let padding_right = Span::styled(
			" ".repeat(self.style.padding[1] as usize),
			self.style.style(),
		);
		let spacing = Span::styled(" ".repeat(self.style.spacing as usize), self.style.style());

		let marker = self.style.markers[self.value as usize].clone();
		let marker = if let Some(style) = self.style.style.clone() {
			marker.style(style)
		} else {
			marker
		};

		let label = self.label.clone();
		let draw = Line::from(vec![padding_left, marker, spacing, label]);

		let mut area = ctx.area;
		area.width -= self.style.padding[1];
		frame.render_widget(draw, area);
		let mut area = ctx.area;
		area.x += ctx.area.width - self.style.padding[1];
		area.width = self.style.padding[1];
		frame.render_widget(padding_right, area);
	}

	fn height(&self) -> u16 {
		1
	}
}
