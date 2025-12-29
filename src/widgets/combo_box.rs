use std::cell::RefCell;
use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
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
				Style::default().bg(Color::Cyan).fg(Color::Black),
				Style::default().bg(Color::Black).fg(Color::White).bold(),
				Style::default().bg(Color::Black).fg(Color::White).italic(),
			],
			completion_selected: [
				Style::default().bg(Color::Cyan).fg(Color::Black),
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
		self.style.unwrap_or_default()
	}

	pub fn style_selected(&self) -> Style {
		match self.selected_style {
			Some(style) => style,
			None => Style::default().fg(Color::Yellow),
		}
	}
}

static DEFAULT_STYLE: LazyLock<ComboBoxStyle> = LazyLock::new(ComboBoxStyle::default);

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

	pub fn style(mut self, style: &'s ComboBoxStyle) -> Self {
		self.style = style;
		self
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

	pub fn submit(&self) -> Option<usize> {
		for ent_id in &self.entries_filter {
			if self.entries[*ent_id].value == self.input {
				return Some(*ent_id);
			}
		}
		None
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
		self.completion_menu = !self.entries_filter.is_empty();
		let found = self
			.entries_filter
			.iter()
			.map(|id| &self.entries[*id])
			.any(|item| item.value == self.input);
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
	fn input(&mut self, key: &KeyEvent) -> bool {
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			// Completion
			KeyCode::Down | KeyCode::Tab if self.completion_menu => self.move_selector(1),
			KeyCode::Char('n') if ctrl_pressed => self.move_selector(1),
			KeyCode::Up | KeyCode::BackTab if self.completion_menu => self.move_selector(-1),
			KeyCode::Char('p') if ctrl_pressed => self.move_selector(1),
			KeyCode::Esc if self.completion_menu => self.completion_menu = false,
			KeyCode::Enter if self.completion_menu => {
				if let Some(index) = self.entries_index {
					self.set_input(self.entries[self.entries_filter[index]].value.clone());
					self.completion_menu = false;
				}
			}
			// Movement
			KeyCode::Left => self.move_cursor_left(),
			KeyCode::Char('b') if ctrl_pressed => self.move_cursor_left(),
			KeyCode::Right => self.move_cursor_right(),
			KeyCode::Char('f') if ctrl_pressed => self.move_cursor_right(),
			KeyCode::Char('a') if ctrl_pressed => {
				self.grapheme_index = 0;
				self.cursor_x = self.cursor_x();
			}
			KeyCode::Char('e') if ctrl_pressed => {
				self.grapheme_index = self.input.len();
				self.cursor_x = self.cursor_x();
			}
			// TODO: Ctrl-arrow and kill-word
			KeyCode::Char(to_insert) if !ctrl_pressed => self.enter_char(to_insert),
			KeyCode::Backspace => self.delete_char(),
			_ => return false,
		}
		true
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
			ctx.set_cursor(Position::new(
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

		let comp_width = std::cmp::min(36, frame.area().width);
		let comp_height = std::cmp::min(
			std::cmp::min(18, frame.area().height - ctx.area.y + 1),
			self.entries_filter.len() as u16,
		);
		let show_scrollbar = (comp_height as usize) < self.entries_filter.len();
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
		if show_scrollbar {
			comp_content.width -= 1;
		}
		let mut buffer = Buffer::empty(comp_area);

		let list = self
			.entries_filter
			.iter()
			.enumerate()
			.map(|(pos, id)| {
				let styles = if Some(pos) == self.entries_index {
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
				let padding = if show_scrollbar {
					Span::styled(" ", styles[1])
				} else {
					Span::default()
				};

				let used_width = (icon.width() + text.width()) as u16;
				let kind_width = kind_span.width() as u16;
				let padding_width = if show_scrollbar {
					(comp_area.width - 2).saturating_sub(used_width + kind_width)
				} else {
					(comp_area.width).saturating_sub(used_width + kind_width)
				};
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

		if show_scrollbar {
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
		}
		ctx.push(Overlay { z_level: 1, buffer });
	}

	fn height(&self) -> u16 {
		1
	}
}
