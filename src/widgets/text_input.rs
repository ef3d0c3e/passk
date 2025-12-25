use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::widgets::widget::WidgetInfo;

pub struct TextInput<'s> {
	title: Line<'s>,
	layout: [Layout; 2],

	input: String,
	character_index: usize,

	active: bool,
}

impl<'s> TextInput<'s> {
	pub fn new(title: Line<'s>, horizontal: Constraint) -> Self {
		Self {
			title,
			layout: [
				Layout::horizontal([horizontal]),
				Layout::vertical([Constraint::Length(3)]),
			],
			input: String::default(),
			character_index: 0,
			active: false,
		}
	}

	pub fn with_input(mut self, input: String) -> Self {
		let len = input.len();
		self.input = input;
		self.character_index = len;
		self
	}

	pub fn set_active(&mut self, active: bool) {
		self.active = active
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

	pub fn input(&mut self, key: &KeyEvent) {
		if !self.active {
			return;
		}
		match key.code {
			KeyCode::Char(to_insert) => self.enter_char(to_insert),
			KeyCode::Backspace => self.delete_char(),
			KeyCode::Left => self.move_cursor_left(),
			KeyCode::Right => self.move_cursor_right(),
			_ => {}
		}
	}

	pub fn draw(&self, frame: &mut Frame, rect: Rect, bg: Option<Color>) {
		let paragraph = Paragraph::new(self.input.as_str())
			.style(if self.active {
				Style::default().fg(Color::Yellow)
			} else {
				Style::default()
			})
			.block(Block::bordered().title(self.title.clone()));
		let area = rect;
		let [area] = area.layout(&self.layout[0]);
		let [area] = area.layout(&self.layout[1]);
		frame.render_widget(Clear, area);
		if let Some(bg) = bg
		{
			frame.render_widget(paragraph.bg(bg), area);
		} else {
			frame.render_widget(paragraph, area);
		}
		if self.active {
			frame.set_cursor_position(Position::new(
				area.x + self.character_index as u16 + 1,
				area.y + 1,
			))
		}
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

impl WidgetInfo for TextInput<'_> {
    fn height(&self) -> u16 {
        3
    }
}
