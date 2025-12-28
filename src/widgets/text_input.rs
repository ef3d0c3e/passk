use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Position;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::Frame;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::widgets::widget::Component;

use super::widget::ComponentRenderCtx;

#[derive(Debug, Clone)]
pub struct TextInputStyle<'s> {
	/// |<padding0><marker0>Input<marker1><padding1>|
	pub padding: [u16; 2],
	pub markers: [Span<'s>; 2],
	/// Style override
	pub style: Option<Style>,
	/// Selected style override
	pub selected_style: Option<Style>,
}

impl Default for TextInputStyle<'_> {
	fn default() -> Self {
		Self {
			padding: Default::default(),
			markers: ["[".into(), "]".into()],
			style: Default::default(),
			selected_style: Default::default(),
		}
	}
}

impl TextInputStyle<'_> {
	pub fn style(&self) -> Style {
		self.style.unwrap_or_default()
	}

	pub fn style_selected(&self) -> Style {
		match self.selected_style {
			Some(style) => style,
			None => Style::default().fg(Color::Yellow),
		}
	}
}

static DEFAULT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(TextInputStyle::default);

pub struct TextInput<'s> {
	input: String,
	grapheme_count: usize,
	grapheme_index: usize,
	cursor_x: u16,

	style: &'s TextInputStyle<'s>,
}

impl<'s> Default for TextInput<'s> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'s> TextInput<'s> {
	pub fn new() -> Self {
		Self {
			input: String::default(),
			grapheme_count: 0,
			grapheme_index: 0,
			cursor_x: 0,
			style: &DEFAULT_STYLE,
		}
	}

	pub fn style(mut self, style: &'s TextInputStyle) -> Self {
		self.style = style;
		self
	}

	pub fn with_input(mut self, input: String) -> Self {
		self.grapheme_count = input.graphemes(true).count();
		self.grapheme_index = self.grapheme_count;
		self.input = input;
		self.cursor_x = self.cursor_x();
		self
	}

	pub fn set_input(&mut self, input: String) {
		self.grapheme_count = input.graphemes(true).count();
		self.grapheme_index = self.grapheme_count;
		self.input = input;
		self.cursor_x = self.cursor_x();
	}

	pub fn submit(&mut self) -> String {
		let mut empty = String::default();
		std::mem::swap(&mut self.input, &mut empty);
		self.grapheme_index = 0;
		self.grapheme_count = 0;
		self.cursor_x = 0;
		empty
	}

	fn move_cursor_left(&mut self) {
		self.grapheme_index = self.grapheme_index.saturating_sub(1);
		self.cursor_x = self.cursor_x();
	}

	fn move_cursor_right(&mut self) {
		self.grapheme_index = std::cmp::min(self.grapheme_index + 1, self.grapheme_count);
		self.cursor_x = self.cursor_x();
	}

	fn enter_char(&mut self, new_char: char) {
		let index: usize = self
			.input
			.graphemes(true)
			.take(self.grapheme_index)
			.map(|g| g.len())
			.sum();
		self.input.insert(index, new_char);
		let prev_count = self.grapheme_count;
		self.grapheme_count = self.input.graphemes(true).count();
		self.cursor_x = self.cursor_x();
		if prev_count != self.grapheme_count {
			self.move_cursor_right()
		}
	}

	fn delete_char(&mut self) {
		if self.grapheme_index == 0 {
			return;
		}

		let start: usize = self
			.input
			.graphemes(true)
			.take(self.grapheme_index - 1)
			.map(|g| g.len())
			.sum();
		let end: usize = self
			.input
			.graphemes(true)
			.take(self.grapheme_index)
			.map(|g| g.len())
			.sum();

		self.input.replace_range(start..end, "");
		self.grapheme_count -= 1;
		self.move_cursor_left();
	}

	fn cursor_x(&self) -> u16 {
		self.input
			.graphemes(true)
			.take(self.grapheme_index)
			.map(|g| UnicodeWidthStr::width(g).max(1))
			.sum::<usize>() as u16
	}
}

impl Component for TextInput<'_> {
	fn input(&mut self, key: &KeyEvent) -> bool {
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			KeyCode::Backspace => self.delete_char(),
			// Movement
			KeyCode::Left => self.move_cursor_left(),
			KeyCode::Char('b') if ctrl_pressed => self.move_cursor_left(),
			KeyCode::Right => self.move_cursor_right(),
			KeyCode::Char('f') if ctrl_pressed => self.move_cursor_right(),
			KeyCode::Char('a') if ctrl_pressed => self.grapheme_index = 0,
			KeyCode::Char('e') if ctrl_pressed => self.grapheme_index = self.input.len(),
			// TODO: Ctrl-arrow and kill-word
			KeyCode::Char(to_insert) => self.enter_char(to_insert),
			_ => return false,
		}
		true
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let padding_left = Span::raw(" ".repeat(self.style.padding[0] as usize));
		let padding_right = Span::raw(" ".repeat(self.style.padding[1] as usize));
		let input_span = Span::from(self.input.as_str());
		let spw = self
			.input
			.graphemes(true)
			.map(|g| UnicodeWidthStr::width(g).max(1))
			.sum::<usize>();
		let empty_space = ctx
			.area
			.width
			.saturating_sub(self.style.padding[0])
			.saturating_sub(self.style.padding[1])
			.saturating_sub(self.style.markers[0].width() as u16)
			.saturating_sub(self.style.markers[1].width() as u16)
			.saturating_sub(spw as u16);
		let spacer = Span::raw(" ".repeat(empty_space as usize));

		let draw = Line::from(vec![
			padding_left,
			self.style.markers[0].clone(),
			input_span,
			spacer,
			self.style.markers[1].clone(),
			padding_right,
		])
		.set_style(if ctx.selected {
			self.style.style_selected()
		} else {
			self.style.style()
		});

		let mut area = ctx.area;
		area.width -= self.style.padding[1];
		frame.render_widget(draw, area);

		if ctx.selected {
			frame.set_cursor_position(Position::new(
				ctx.area.x
					+ self.cursor_x + self.style.padding[0]
					+ self.style.markers[0].width() as u16,
				ctx.area.y,
			))
		}
	}

	fn height(&self) -> u16 {
		1
	}
}
