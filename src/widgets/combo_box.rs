use std::cell::RefCell;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::Frame;

pub struct ComboItem {
	pub kind: String,
	pub icon: String,
	pub value: String,
}

pub struct ComboBox<'s, 'e> {
	title: Line<'s>,
	layout: [Layout; 2],
	completion_width: Layout,

	entries: &'e [ComboItem],
	entries_filter: Vec<usize>,
	input: String,
	character_index: usize,
	entries_index: i32,
	list_state: RefCell<ListState>,
	scrollbar: RefCell<ScrollbarState>,

	active: bool,
	completion_menu: bool,
}

impl<'s, 'e> ComboBox<'s, 'e> {
	pub fn new(title: Line<'s>, horizontal: Constraint, entries: &'e [ComboItem]) -> Self {
		let num_entries = entries.len();
		Self {
			title,
			layout: [
				Layout::horizontal([horizontal]),
				Layout::vertical([Constraint::Length(3)]),
			],
			completion_width: Layout::horizontal([Constraint::Max(40)]),
			entries,
			entries_filter: (0..num_entries).collect(),
			input: String::default(),
			character_index: 0,
			entries_index: -1,
			list_state: RefCell::default(),
			scrollbar: RefCell::new(ScrollbarState::new(num_entries).position(0)),
			active: false,
			completion_menu: true,
		}
	}

	pub fn with_input(mut self, input: String) -> Self {
		let len = input.len();
		self.input = input;
		self.character_index = len;
		self
	}

	pub fn set_active(&mut self, active: bool) {
		self.active = active;
		if active {
			self.update_filer();
		}
	}

	pub fn is_active(&self) -> bool {
		self.active
	}

	pub fn is_completing(&self) -> bool {
		self.completion_menu
	}

	pub fn set_input(&mut self, input: String) {
		self.character_index = input.len();
		self.input = input;
	}

	pub fn submit(&mut self) -> Option<usize> {
		for i in &self.entries_filter {
			if self.entries[*i].value == self.input {
				return Some(*i)
			}
		}
		None
	}

	fn update_filer(&mut self) {
		self.entries_index = -1;
		self.entries_filter.clear();
		self.entries_filter.reserve(self.entries.len());
		let filter_low = self.input.to_lowercase();
		self.entries.iter().enumerate().for_each(|(id, ent)| {
			if ent.value.to_lowercase().contains(&filter_low) {
				self.entries_filter.push(id);
			}
		});
		self.completion_menu = true;
		let found = self
			.entries_filter
			.iter()
			.map(|id| &self.entries[*id])
			.fold(false, |r, item| r || item.value == self.input);
		if found {
			self.completion_menu = false;
		}
		*self.scrollbar.borrow_mut() = ScrollbarState::new(self.entries_filter.len()).position(0);
		self.list_state.borrow_mut().select(None);
	}

	pub fn input(&mut self, key: &KeyEvent) {
		if !self.active {
			return;
		}
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			// Completion menu
			KeyCode::Down if self.completion_menu => {
				self.entries_index = std::cmp::min(
					self.entries_index + 1,
					self.entries_filter.len().saturating_sub(1) as i32,
				);
				if self.entries_filter.is_empty() {
					self.entries_index = -1
				} else {
					self.list_state
						.borrow_mut()
						.select(Some(self.entries_index as usize));
					let sc = self
						.scrollbar
						.borrow_mut()
						.position(self.entries_index as usize);
					*self.scrollbar.borrow_mut() = sc
				}
			}
			KeyCode::Char('n') if ctrl_pressed && self.completion_menu => {
				self.entries_index = std::cmp::min(
					self.entries_index + 1,
					self.entries_filter.len().saturating_sub(1) as i32,
				);
				if self.entries_filter.is_empty() {
					self.entries_index = -1
				} else {
					self.list_state
						.borrow_mut()
						.select(Some(self.entries_index as usize));
					let sc = self
						.scrollbar
						.borrow_mut()
						.position(self.entries_index as usize);
					*self.scrollbar.borrow_mut() = sc
				}
			}
			KeyCode::PageDown if self.completion_menu => {
				self.entries_index = std::cmp::min(
					self.entries_index + 16,
					self.entries_filter.len().saturating_sub(1) as i32,
				);
				if self.entries_filter.is_empty() {
					self.entries_index = -1
				} else {
					self.list_state
						.borrow_mut()
						.select(Some(self.entries_index as usize));
					let sc = self
						.scrollbar
						.borrow_mut()
						.position(self.entries_index as usize);
					*self.scrollbar.borrow_mut() = sc
				}
			}
			KeyCode::Up if self.completion_menu => {
				self.entries_index = std::cmp::max(self.entries_index - 1, 0);
				if self.entries_filter.is_empty() {
					self.entries_index = -1
				} else {
					self.list_state
						.borrow_mut()
						.select(Some(self.entries_index as usize));
					let sc = self
						.scrollbar
						.borrow_mut()
						.position(self.entries_index as usize);
					*self.scrollbar.borrow_mut() = sc
				}
			}
			KeyCode::Char('p') if ctrl_pressed && self.completion_menu => {
				self.entries_index = std::cmp::max(self.entries_index - 1, 0);
				if self.entries_filter.is_empty() {
					self.entries_index = -1
				} else {
					self.list_state
						.borrow_mut()
						.select(Some(self.entries_index as usize));
					let sc = self
						.scrollbar
						.borrow_mut()
						.position(self.entries_index as usize);
					*self.scrollbar.borrow_mut() = sc
				}
			}
			KeyCode::PageUp if self.completion_menu => {
				self.entries_index = std::cmp::max(self.entries_index - 16, 0);
				if self.entries_filter.is_empty() {
					self.entries_index = -1
				} else {
					self.list_state
						.borrow_mut()
						.select(Some(self.entries_index as usize));
					let sc = self
						.scrollbar
						.borrow_mut()
						.position(self.entries_index as usize);
					*self.scrollbar.borrow_mut() = sc
				}
			}

			KeyCode::Esc => {
				self.completion_menu = false
			}
			KeyCode::Enter | KeyCode::Tab if self.entries_index != -1 && self.completion_menu => {
				self.input = self.entries[self.entries_filter[self.entries_index as usize]]
					.value
					.clone();
				self.character_index = self.input.len();
				self.entries_index = -1;
				self.completion_menu = false;
			}

			KeyCode::Char(to_insert) => {
				self.enter_char(to_insert);
				self.update_filer();
			}
			KeyCode::Backspace => {
				self.delete_char();
				self.update_filer();
			}
			KeyCode::Left => self.move_cursor_left(),
			KeyCode::Right => self.move_cursor_right(),
			_ => {}
		}
	}

	pub fn draw(&self, frame: &mut Frame, rect: Rect) {
		let popup = Paragraph::new(self.input.as_str())
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
		frame.render_widget(popup, area);
		if self.active {
			frame.set_cursor_position(Position::new(
				area.x + self.character_index as u16 + 1,
				area.y + 1,
			))
		}

		// Dropdown indicator
		let indicator = Span::from(["", " "][(self.completion_menu == true) as usize]);
		let indicator_area = Rect {
			x: area.x + area.width - 3,
			y: area.y + 1,
			width: 2,
			height: 1,
		};
		frame.render_widget(indicator, indicator_area);

		// Dropdown
		if self.entries_filter.is_empty() || !self.active || !self.completion_menu {
			return;
		}

		let comp_area = frame.area();
		let [width] = area.layout(&self.completion_width);
		let width = width.width;
		let area = Rect {
			x: area.x + 1,
			y: area.y + 2,
			width: width - 1,
			height: comp_area.height - area.y - 2,
		};
		let height = std::cmp::min(self.entries_filter.len() as u16, area.height);
		let area_scrollbar = Rect {
			x: area.x + width - 1,
			y: area.y,
			width: 1,
			height,
		};
		let list = self
			.entries_filter
			.iter()
			.map(|id| {
				let ent = &self.entries[*id];
				// Icon
				let icon = Line::from(vec![
					" ".fg(Color::Black).bg(Color::Cyan),
					ent.icon.as_str().fg(Color::Black).bg(Color::Cyan),
					" ".fg(Color::Black).bg(Color::Cyan),
				]);

				// Value
				let text = Line::from(vec![
					" ".bg(Color::Black),
					ent.value
						.as_str()
						.bg(Color::Black)
						.fg(
							if self.entries_index != -1
								&& *id == self.entries_filter[self.entries_index as usize]
							{
								Color::Yellow
							} else {
								Color::White
							},
						)
						.bold(),
				]);

				// Kind
				let kind_span = Span::styled(
					ent.kind.as_str(),
					ratatui::style::Style::default()
						.fg(Color::Gray)
						.bg(Color::Black)
						.italic(),
				);

				// Padding
				let padding = Span::styled(
					" ",
					ratatui::style::Style::default().bg(Color::Black).italic(),
				);

				let used_width = (icon.width() + text.width()) as u16;
				let kind_width = kind_span.width() as u16;
				let padding_width = (width - 2).saturating_sub(used_width + kind_width);
				let spacer = Span::styled(
					" ".repeat(padding_width as usize),
					ratatui::style::Style::default().bg(Color::Black),
				);

				let line = Line::from(
					vec![
						icon.spans,
						text.spans,
						vec![spacer],
						vec![kind_span],
						vec![padding],
					]
					.into_iter()
					.flatten()
					.collect::<Vec<_>>(),
				);
				ListItem::new(line)
			})
			.collect::<Vec<_>>();
		// Render list
		let list = List::new(list);
		frame.render_stateful_widget(list, area, &mut self.list_state.borrow_mut());
		// Render scrollbar
		let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
			.begin_symbol(None)
			.end_symbol(None)
			.track_symbol(Some(" "))
			.track_style(Style::default().bg(Color::Black))
			.thumb_symbol("█")
			.thumb_style(Style::default().fg(Color::White));
		frame.render_stateful_widget(scrollbar, area_scrollbar, &mut self.scrollbar.borrow_mut());
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
