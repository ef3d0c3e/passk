use std::cell::RefCell;
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
	pub style_selected: Option<Style>,
}

impl Default for TextInputStyle<'_> {
	fn default() -> Self {
		Self {
			padding: Default::default(),
			markers: ["[".into(), "]".into()],
			style: Default::default(),
			style_selected: Default::default(),
		}
	}
}

impl TextInputStyle<'_> {
	pub fn style(&self) -> Style {
		self.style.unwrap_or_default()
	}

	pub fn style_selected(&self) -> Style {
		match self.style_selected {
			Some(style) => style,
			None => Style::default().fg(Color::Yellow),
		}
	}
}

static DEFAULT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(TextInputStyle::default);

pub struct TextInput<'s> {
	style: &'s TextInputStyle<'s>,

	input: String,
	grapheme_index: usize,

	cursor_x: u16,
	scroll_x: RefCell<u16>,
	geometry: Vec<u16>,
}

impl<'s> Default for TextInput<'s> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'s> TextInput<'s> {
	pub fn new() -> Self {
		Self {
			style: &DEFAULT_STYLE,
			input: String::default(),
			grapheme_index: 0,
			cursor_x: 0,
			scroll_x: RefCell::default(),
			geometry: vec![],
		}
	}

	pub fn style(mut self, style: &'s TextInputStyle) -> Self {
		self.style = style;
		self
	}

	pub fn with_input(mut self, input: String) -> Self {
		self.input = input;
		self.rebuild_geometry();
		self.grapheme_index = self.geometry.len();
		self.update_cursor_x();
		self
	}

	pub fn set_input(&mut self, input: String) {
		self.input = input;
		self.rebuild_geometry();
		self.grapheme_index = self.geometry.len();
		self.update_cursor_x();
	}

	pub fn get_input(&self) -> &String {
		&self.input
	}

	pub fn submit(&self) -> String {
		self.input.clone()
	}

	fn move_cursor_left(&mut self) {
		self.grapheme_index = self.grapheme_index.saturating_sub(1);
		self.update_cursor_x();
	}

	fn move_cursor_right(&mut self) {
		self.grapheme_index = std::cmp::min(self.grapheme_index + 1, self.geometry.len());
		self.update_cursor_x();
	}

	fn enter_char(&mut self, new_char: char) {
		let index: usize = self
			.input
			.graphemes(true)
			.take(self.grapheme_index)
			.map(|g| g.len())
			.sum();
		self.input.insert(index, new_char);
		let prev_count = self.geometry.len();
		self.rebuild_geometry();
		self.update_cursor_x();
		if prev_count != self.geometry.len() {
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
		self.rebuild_geometry();
		self.move_cursor_left();
	}

	fn update_cursor_x(&mut self) {
		self.cursor_x = self.geometry[..self.grapheme_index].iter().copied().sum();
	}

	fn rebuild_geometry(&mut self) {
		self.geometry = self
			.input
			.graphemes(true)
			.map(|g| UnicodeWidthStr::width(g).max(1) as u16)
			.collect();
	}

	/// Width taken by text in the current viewport
	fn text_width(&self, viewport_width: u16) -> u16 {
		viewport_width
			- self.style.padding[0]
			- self.style.padding[1]
			- self.style.markers[0].width() as u16
			- self.style.markers[1].width() as u16
	}

	/// Update scroll so that it's position is visible
	fn ensure_cursor_visible(&self, viewport_width: u16) {
		let mut scroll_x = *self.scroll_x.borrow();

		if self.cursor_x < scroll_x {
			scroll_x = self.cursor_x;
		}
		if self.cursor_x >= scroll_x + viewport_width {
			scroll_x = self.cursor_x + 1 - viewport_width;
		}

		let desired = scroll_x;
		scroll_x = 0;
		let grapheme_columns = self.geometry.iter().scan(0, |col, w| {
			let start = *col;
			*col += *w;
			Some(start)
		});
		for col in grapheme_columns {
			if col > desired {
				break;
			}
			scroll_x = col;
		}

		*self.scroll_x.borrow_mut() = scroll_x;
	}

	/// Get a span of all graphemes visible inside the viewport
	fn visible_graphemes(&self, viewport_width: u16) -> Vec<Span<'_>> {
		let scroll_x = *self.scroll_x.borrow();
		let mut spans = Vec::new();
		let mut col = 0;

		for (g, w) in self.input.graphemes(true).zip(self.geometry.iter()) {
			let next_col = col + *w;
			// Skip
			if next_col <= scroll_x {
				col = next_col;
				continue;
			}
			// Stop
			if col >= scroll_x + viewport_width {
				break;
			}

			spans.push(Span::raw(g.to_string()));
			col = next_col;
		}

		spans
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
			KeyCode::Char('a') if ctrl_pressed => {
				self.grapheme_index = 0;
				self.update_cursor_x();
			}
			KeyCode::Char('e') if ctrl_pressed => {
				self.grapheme_index = self.geometry.len();
				self.update_cursor_x();
			}
			// TODO: Ctrl-arrow and kill-word
			KeyCode::Char(to_insert) if !ctrl_pressed => self.enter_char(to_insert),
			_ => return false,
		}
		true
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let viewport_width = self.text_width(ctx.area.width);
		self.ensure_cursor_visible(viewport_width);

		let padding_left = Span::raw(" ".repeat(self.style.padding[0] as usize));
		let padding_right = Span::raw(" ".repeat(self.style.padding[1] as usize));
		let visible = self.visible_graphemes(viewport_width);
		let empty_space =
			viewport_width.saturating_sub(visible.iter().map(|sp| sp.width() as u16).sum());
		let spacer = Span::raw(" ".repeat(empty_space as usize));

		let mut comps = vec![padding_left, self.style.markers[0].clone()];
		comps.extend_from_slice(visible.as_slice());
		comps.push(spacer);
		comps.push(self.style.markers[1].clone());
		comps.push(padding_right);
		let draw = Line::from(comps).set_style(if ctx.selected {
			self.style.style_selected()
		} else {
			self.style.style()
		});

		let mut area = ctx.area;
		area.width -= self.style.padding[1];
		frame.render_widget(draw, area);

		if ctx.selected {
			ctx.set_cursor(Position::new(
				ctx.area.x
					+ self.style.padding[0]
					+ self.style.markers[0].width() as u16
					+ self.cursor_x.saturating_sub(*self.scroll_x.borrow()),
				ctx.area.y,
			));
		}
	}

	fn height(&self) -> u16 {
		1
	}
}
