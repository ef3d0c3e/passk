use std::sync::LazyLock;

use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

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

static DEFAULT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(|| TextInputStyle::default());

pub struct TextInput<'s> {
	input: String,
	character_index: usize,

	style: &'s TextInputStyle<'s>,
}

impl<'s> TextInput<'s> {
	pub fn new() -> Self {
		Self {
			input: String::default(),
			character_index: 0,
			style: &DEFAULT_STYLE,
		}
	}

	pub fn with_input(mut self, input: String) -> Self {
		let len = input.len();
		self.input = input;
		self.character_index = len;
		self
	}

	pub fn set_input(&mut self, input: String) {
		self.character_index = input.len();
		self.input = input;
	}

	pub fn submit(&mut self) -> String {
		let mut empty = String::default();
		std::mem::swap(&mut self.input, &mut empty);
		self.character_index = 0;
		empty
	}

	fn move_cursor_left(&mut self) {
		let cursor_moved_left = self.character_index.saturating_sub(1);
		self.character_index = self.clamp_cursor(cursor_moved_left);
	}

	fn move_cursor_right(&mut self) {
		let cursor_moved_right = self.character_index.saturating_add(1);
		self.character_index = self.clamp_cursor(cursor_moved_right);
	}

	fn enter_char(&mut self, new_char: char) {
		let index = self.byte_index();
		self.input.insert(index, new_char);
		self.move_cursor_right();
	}

	/// Returns the byte index based on the character position.
	///
	/// Since each character in a string can be contain multiple bytes, it's necessary to calculate
	/// the byte index based on the index of the character.
	fn byte_index(&self) -> usize {
		self.input
			.char_indices()
			.map(|(i, _)| i)
			.nth(self.character_index)
			.unwrap_or(self.input.len())
	}

	fn delete_char(&mut self) {
		let is_not_cursor_leftmost = self.character_index != 0;
		if is_not_cursor_leftmost {
			// Method "remove" is not used on the saved text for deleting the selected char.
			// Reason: Using remove on String works on bytes instead of the chars.
			// Using remove would require special care because of char boundaries.

			let current_index = self.character_index;
			let from_left_to_current_index = current_index - 1;

			// Getting all characters before the selected character.
			let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
			// Getting all characters after selected character.
			let after_char_to_delete = self.input.chars().skip(current_index);

			// Put all characters together except the selected one.
			// By leaving the selected one out, it is forgotten and therefore deleted.
			self.input = before_char_to_delete.chain(after_char_to_delete).collect();
			self.move_cursor_left();
		}
	}

	fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
		new_cursor_pos.clamp(0, self.input.chars().count())
	}
}

impl Component for TextInput<'_> {
	fn input(&mut self, key: &KeyEvent) {
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			KeyCode::Backspace => self.delete_char(),
			// Movement
			KeyCode::Left => self.move_cursor_left(),
			KeyCode::Char('b') if ctrl_pressed => self.move_cursor_left(),
			KeyCode::Right => self.move_cursor_right(),
			KeyCode::Char('f') if ctrl_pressed => self.move_cursor_right(),
			KeyCode::Char('a') if ctrl_pressed => self.character_index = 0,
			KeyCode::Char('e') if ctrl_pressed => self.character_index = self.input.len(),
			// TODO: Ctrl-arrow and kill-word
			KeyCode::Char(to_insert) => self.enter_char(to_insert),
			_ => {}
		}
	}

	fn render(&self, frame: &mut Frame, ctx: &ComponentRenderCtx) {
		let padding_left = Span::raw(" ".repeat(self.style.padding[0] as usize));
		let padding_right = Span::raw(" ".repeat(self.style.padding[1] as usize));
		let input_span = Span::from(self.input.as_str());
		let empty_space = ctx.area.width
			- self.style.padding[0]
			- self.style.padding[1]
			- self.style.markers[0].width() as u16
			- self.style.markers[1].width() as u16
			- input_span.width() as u16;
		let spacer = Span::raw(" ".repeat(empty_space as usize));

		let draw = Line::from(vec![
			padding_left,
			self.style.markers[0].clone(),
			input_span,
			spacer,
			self.style.markers[1].clone(),
		])
		.set_style(if ctx.selected {
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
		frame.render_widget(
			padding_right.set_style(if ctx.selected {
				self.style.style_selected()
			} else {
				self.style.style()
			}),
			area,
		);

		if ctx.selected {
			frame.set_cursor_position(Position::new(
				ctx.area.x + self.character_index as u16 + self.style.markers[0].width() as u16,
				ctx.area.y,
			))
		}
	}

	fn height(&self) -> u16 {
		1
	}
}
