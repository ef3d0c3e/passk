use std::cell::RefCell;
use std::ops::SubAssign;
use std::sync::LazyLock;

use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::buffer::Cell;
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
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::Frame;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::widgets::widget::Component;
use crate::widgets::widget::Overlay;

use super::widget::ComponentRenderCtx;

pub struct ComboItem {
	pub kind: String,
	pub icon: String,
	pub value: String,
}

#[derive(Debug, Clone)]
pub struct ComboBoxStyle<'s> {
	/// |<padding0><marker0>Input<indicator><marker1><padding1>|
	pub padding: [u16; 2],
	pub markers: [Span<'s>; 2],
	pub indicator: [Span<'s>; 2],

	pub completion: [Style; 3],
	pub completion_selected: [Style; 3],

	/// Style override
	pub style: Option<Style>,
	/// Selected style override
	pub selected_style: Option<Style>,
}

impl Default for ComboBoxStyle<'_> {
	fn default() -> Self {
		Self {
			padding: Default::default(),
			markers: ["[".into(), "]".into()],
			indicator: [" ".into(), " ".into()],
			completion: [
				Style::default().bg(Color::Cyan).fg(Color::White),
				Style::default().bg(Color::Black).fg(Color::White).bold(),
				Style::default().bg(Color::Black).fg(Color::White).italic(),
			],
			completion_selected: [
				Style::default().bg(Color::Cyan).fg(Color::White),
				Style::default().bg(Color::Black).fg(Color::Yellow).bold(),
				Style::default().bg(Color::Black).fg(Color::Yellow).italic(),
			],
			style: Default::default(),
			selected_style: Default::default(),
		}
	}
}

impl ComboBoxStyle<'_> {
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

static DEFAULT_STYLE: LazyLock<ComboBoxStyle> = LazyLock::new(|| ComboBoxStyle::default());

pub struct ComboBox<'s, 'e> {
	style: &'s ComboBoxStyle<'s>,

	input: String,
	grapheme_count: usize,
	grapheme_index: usize,
	cursor_x: u16,

	/// Entries
	entries: &'e [ComboItem],
	/// Filtered entries
	entries_filter: Vec<usize>,
	/// Position in the completion menu
	entries_index: Option<usize>,
	/// Whether completion menu is shown
	completion_menu: bool,

	list_state: RefCell<ListState>,
	scrollbar: RefCell<ScrollbarState>,
}

impl<'s, 'e> ComboBox<'s, 'e> {
	pub fn new(entries: &'e [ComboItem]) -> Self {
		let num_entries = entries.len();
		Self {
			style: &DEFAULT_STYLE,
			input: String::default(),
			grapheme_count: 0,
			grapheme_index: 0,
			cursor_x: 0,

			entries,
			entries_filter: (0..num_entries).collect(),
			entries_index: None,
			completion_menu: false,

			list_state: RefCell::default(),
			scrollbar: RefCell::new(ScrollbarState::new(num_entries).position(0)),
		}
	}

	pub fn with_input(mut self, input: String) -> Self {
		self.grapheme_count = input.graphemes(true).count();
		self.grapheme_index = self.grapheme_count;
		self.input = input;
		self.cursor_x = self.cursor_x();
		self.update_filter();
		self
	}

	pub fn set_input(&mut self, input: String) {
		self.grapheme_count = input.graphemes(true).count();
		self.grapheme_index = self.grapheme_count;
		self.input = input;
		self.cursor_x = self.cursor_x();
		self.update_filter();
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
		self.update_filter();
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
		self.update_filter();
	}

	fn move_selector(&mut self, offset: i32) {
		if offset > 0 {
			if self.entries_filter.is_empty() {
				self.entries_index = None
			} else {
				let index = if let Some(index) = self.entries_index {
					std::cmp::min(
						index + offset as usize,
						self.entries_filter.len().saturating_sub(1),
					)
				} else {
					std::cmp::min(
						(offset as usize).saturating_sub(1),
						self.entries_filter.len().saturating_sub(1),
					)
				};
				self.entries_index = Some(index);
				self.list_state.borrow_mut().select(Some(index));
				let sc = self.scrollbar.borrow_mut().position(index);
				*self.scrollbar.borrow_mut() = sc
			}
		} else if offset < 0 {
			if self.entries_filter.is_empty() {
				self.entries_index = None
			} else {
				let index = if let Some(index) = self.entries_index {
					if index < (-offset) as usize {
						Some(0)
					} else {
						Some(index - (-offset) as usize)
					}
				} else {
					None
				};

				self.entries_index = index;
				self.list_state.borrow_mut().select(index);
				if let Some(index) = index {
					let sc = self.scrollbar.borrow_mut().position(index);
					*self.scrollbar.borrow_mut() = sc
				}
			}
		}
	}

	fn update_filter(&mut self) {
		self.entries_index = None;
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

	fn cursor_x(&self) -> u16 {
		self.input
			.graphemes(true)
			.take(self.grapheme_index)
			.map(|g| UnicodeWidthStr::width(g).max(1))
			.sum::<usize>() as u16
	}
}

impl Component for ComboBox<'_, '_> {
	fn input(&mut self, key: &KeyEvent) {
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			// Completion
			KeyCode::End if self.completion_menu => self.move_selector(1),
			KeyCode::Home if self.completion_menu => self.move_selector(-1),
			// Movement
			KeyCode::Left => self.move_cursor_left(),
			KeyCode::Char('b') if ctrl_pressed => self.move_cursor_left(),
			KeyCode::Right => self.move_cursor_right(),
			KeyCode::Char('f') if ctrl_pressed => self.move_cursor_right(),
			KeyCode::Char('a') if ctrl_pressed => self.grapheme_index = 0,
			KeyCode::Char('e') if ctrl_pressed => self.grapheme_index = self.input.len(),
			// TODO: Ctrl-arrow and kill-word
			KeyCode::Char(to_insert) => self.enter_char(to_insert),
			KeyCode::Backspace => self.delete_char(),
			_ => {}
		}
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let padding_left = Span::raw(" ".repeat(self.style.padding[0] as usize));
		let padding_right = Span::raw(" ".repeat(self.style.padding[1] as usize));
		let input_span = Span::from(self.input.as_str());
		let indicator = self.style.indicator[self.completion_menu as usize].clone();

		let left = Rect {
			x: ctx.area.x,
			y: ctx.area.y,
			width: ctx
				.area
				.width
				.saturating_sub(padding_right.width() as u16)
				.saturating_sub(self.style.markers[1].width() as u16)
				.saturating_sub(indicator.width() as u16),
			height: ctx.area.height,
		};
		let right = Rect {
			x: left.x + left.width,
			y: left.y,
			width: ctx.area.width.saturating_sub(left.width),
			height: left.height,
		};

		let draw_left = Line::from(vec![
			padding_left,
			self.style.markers[0].clone(),
			input_span,
		])
		.set_style(if ctx.selected {
			self.style.style_selected()
		} else {
			self.style.style()
		});

		let draw_right = Line::from(vec![
			indicator,
			self.style.markers[1].clone(),
			padding_right,
		])
		.set_style(if ctx.selected {
			self.style.style_selected()
		} else {
			self.style.style()
		});

		frame.render_widget(draw_left, left);
		frame.render_widget(draw_right, right);

		if ctx.selected {
			frame.set_cursor_position(Position::new(
				ctx.area.x
					+ self.cursor_x + self.style.padding[0]
					+ self.style.markers[0].width() as u16,
				ctx.area.y,
			))
		}

		// Completion menu
		if !self.completion_menu {
			return;
		}

		let comp_width = std::cmp::min(24, frame.area().width);
		let comp_height = std::cmp::min(
			std::cmp::min(3, frame.area().height - ctx.area.y + 1),
			self.entries.len() as u16,
		);
		let mut comp_x = ctx.area.x
			+ self.cursor_x
			+ self.style.padding[0]
			+ self.style.markers[0].width() as u16;
		comp_x = std::cmp::min(comp_x, frame.area().width.saturating_sub(comp_width));
		let comp_area = Rect {
			x: comp_x,
			y: ctx.area.y + 1,
			width: comp_width,
			height: comp_height,
		};
		let mut comp_content = comp_area;
		comp_content.width -= 2;
		let mut buffer = Buffer::empty(comp_area);

		let list = self
			.entries_filter
			.iter()
			.map(|id| {
				let styles = if Some(*id) == self.entries_index {
					&self.style.completion_selected
				} else {
					&self.style.completion
				};

				let ent = &self.entries[*id];
				// Icon
				let icon = Line::from(vec![
					Span::from(" ").style(styles[0]),
					Span::from(ent.icon.as_str()).style(styles[0]),
					Span::from(" ").style(styles[0]),
				]);

				// Value
				let text = Line::from(vec![
					Span::from(" ").style(styles[1]),
					Span::from(ent.value.as_str()).style(styles[1]),
				]);

				// Kind
				let kind_span = Span::from(ent.kind.as_str()).style(styles[2]);

				// Padding
				let padding = Span::styled(" ", styles[1]);

				let used_width = (icon.width() + text.width()) as u16;
				let kind_width = kind_span.width() as u16;
				let padding_width = (comp_area.width - 2).saturating_sub(used_width + kind_width);
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
		StatefulWidget::render(
			List::new(list),
			comp_content,
			&mut buffer,
			&mut self.list_state.borrow_mut(),
		);

		// Scrollbar
		let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
			.begin_symbol(None)
			.end_symbol(None)
			.track_symbol(Some(" "))
			.track_style(Style::default().bg(Color::Black))
			.thumb_symbol("█")
			.thumb_style(Style::default().fg(Color::White));
		let mut comp_scrollbar = comp_area;
		comp_scrollbar.x += comp_scrollbar.width.saturating_sub(2);
		comp_scrollbar.width = 2;
		StatefulWidget::render(
			scrollbar,
			comp_scrollbar,
			&mut buffer,
			&mut self.scrollbar.borrow_mut(),
		);
		ctx.push(Overlay { z_level: 1, buffer });
	}

	fn height(&self) -> u16 {
		1
	}
}

/*
pub struct ComboBox<'s, 'e> {
	title: Line<'s>,
	layout: [Layout; 2],
	completion_width: Layout,

	input: String,
	character_index: usize,

	entries: &'e [ComboItem],
	entries_filter: Vec<usize>,
	entries_index: i32,
	completion_menu: bool,
	list_state: RefCell<ListState>,
	scrollbar: RefCell<ScrollbarState>,
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
				return Some(*i);
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

			KeyCode::Esc => self.completion_menu = false,
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

	pub fn draw(&self, frame: &mut Frame, rect: Rect, bg: Option<Color>) {
		let text_input = Paragraph::new(self.input.as_str())
			.style(if self.active {
				Style::default().fg(Color::Yellow)
			} else {
				Style::default()
			})
			.block(Block::bordered().title(self.title.clone()));
		let area = rect;
		let [area] = area.layout(&self.layout[0]);
		let [area] = area.layout(&self.layout[1]);
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

		// Draw
		frame.render_widget(Clear, area);
		if let Some(bg) = bg {
			frame.render_widget(text_input.bg(bg), area);
			frame.render_widget(indicator.bg(bg), indicator_area);
		} else {
			frame.render_widget(text_input, area);
			frame.render_widget(indicator, indicator_area);
		}

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
*/
