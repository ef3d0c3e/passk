use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::text::Span;
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
	/// Selected style override
	pub selected_style: Option<Style>,
}

impl Default for CheckboxStyle<'_> {
	fn default() -> Self {
		Self {
			padding: Default::default(),
			spacing: 1,
			markers: ["[ ]".into(), "[x]".into()],
			style: Default::default(),
			selected_style: Default::default(),
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

	pub fn style_selected(&self) -> Style {
		match self.selected_style {
			Some(style) => style.clone(),
			None => Style::default().fg(Color::Yellow),
		}
	}
}

static DEFAULT_STYLE: LazyLock<CheckboxStyle> = LazyLock::new(|| CheckboxStyle::default());

pub struct Checkbox<'s> {
	value: bool,
	style: &'s CheckboxStyle<'s>,

	label: Span<'s>,
}

impl<'s> Checkbox<'s> {
	pub fn new(value: bool, label: Span<'s>) -> Self {
		Self {
			value,
			style: &DEFAULT_STYLE,
			label,
		}
	}

	pub fn style(mut self, style: &'s CheckboxStyle<'s>) -> Self {
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

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let padding_left = Span::raw(" ".repeat(self.style.padding[0] as usize));
		let padding_right = Span::raw(" ".repeat(self.style.padding[1] as usize));
		let spacing = Span::raw(" ".repeat(self.style.spacing as usize));

		let marker = self.style.markers[self.value as usize].clone();
		let label = self.label.clone();

		let draw =
			Line::from(vec![padding_left, marker, spacing, label]).set_style(if ctx.selected {
				self.style.style_selected()
			} else {
				self.style.style()
			});

		let mut area = ctx.area;
		area.width -= self.style.padding[1];
		frame.render_widget(draw, area);
		let mut area = ctx.area;
		area.x += ctx.area.width - self.style.padding[1];
		area.width = self.style.padding[1];
		area.width = self.style.padding[1];
		frame.render_widget(
			padding_right.set_style(if ctx.selected {
				self.style.style_selected()
			} else {
				self.style.style()
			}),
			area,
		);
	}

	fn height(&self) -> u16 {
		1
	}
}
