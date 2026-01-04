use std::cell::Cell;
use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::HorizontalAlignment;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;

use crate::widgets::widget::Component;

use super::widget::ComponentRenderCtx;

#[derive(Clone)]
pub struct ConfirmStyle<'s> {
	padding: [u16; 4],
	block: Block<'s>,
	buttons: [Style; 2],
	spacing: u16,
}

impl Default for ConfirmStyle<'_> {
	fn default() -> Self {
		Self {
			padding: [0, 1, 0, 1],
			block: Block::bordered()
				.bg(Color::from_u32(0x1f1f1f))
				.title_alignment(HorizontalAlignment::Center),
			buttons: [
				Style::default().fg(Color::White),
				Style::default().bg(Color::White).fg(Color::Black).bold(),
			],
			spacing: 2,
		}
	}
}

static DEFAULT_STYLE: LazyLock<ConfirmStyle> = LazyLock::new(ConfirmStyle::default);

#[derive(Clone, Copy)]
struct HeightCache {
	width: u16,
	height: u16,
}

pub struct Confirm<'s> {
	style: &'s ConfirmStyle<'s>,
	title: String,
	content: Paragraph<'s>,
	layout: Layout,
	selected: usize,
	submit: Option<bool>,

	cached_height: Cell<Option<HeightCache>>,
}

impl<'s> Confirm<'s> {
	pub fn new(title: String, content: Paragraph<'s>) -> Self {
		let horizontal = Layout::horizontal([Constraint::Percentage(30)]).flex(Flex::Center);
		Self {
			style: &DEFAULT_STYLE,
			title,
			content,
			layout: horizontal,
			selected: 0,
			submit: None,
			cached_height: Cell::default(),
		}
	}

	pub fn style(mut self, style: &'s ConfirmStyle) -> Self {
		self.style = style;
		self
	}

	pub fn set_selected(&mut self, selected: usize) {
		assert!(selected <= 1);
		self.selected = selected;
	}

	fn measured_height(&self, width: u16, max_height: u16) -> u16 {
		fn measure_paragraph_height_fast(
			paragraph: &Paragraph,
			width: u16,
			max_height: u16,
		) -> u16 {
			if width == 0 || max_height == 0 {
				return 0;
			}

			let area = Rect::new(0, 0, width, max_height);
			let mut buffer = Buffer::empty(area);

			paragraph.render(area, &mut buffer);

			let mut last_used_row: i16 = -1;

			for y in 0..max_height {
				if last_used_row >= 0 && y > last_used_row as u16 + 1 {
					break;
				}

				let mut row_used = false;

				for x in 0..width {
					if buffer[(x, y)].symbol() != " " {
						row_used = true;
						break;
					}
				}

				if row_used {
					last_used_row = y as i16;
				}
			}

			(last_used_row + 1) as u16
		}

		if let Some(cache) = self.cached_height.take() {
			if cache.width == width {
				self.cached_height.set(Some(cache));
				return cache.height;
			}
		}

		let height = measure_paragraph_height_fast(&self.content, width, max_height);
		self.cached_height.set(Some(HeightCache { width, height }));

		height
	}

	pub fn submit(&self) -> Option<bool> {
		self.submit
	}
}

impl Component for Confirm<'_> {
	fn input(&mut self, key: &KeyEvent) -> bool {
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			// Movement
			KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
				self.selected = std::cmp::min(1, self.selected + 1)
			}
			KeyCode::Char('n') if ctrl_pressed => self.selected = std::cmp::min(1, self.selected + 1),
			KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
				self.selected = self.selected.saturating_sub(1)
			}
			KeyCode::Char('p') if ctrl_pressed => self.selected = self.selected.saturating_sub(1),

			// Validate
			KeyCode::Char('y') => self.submit = Some(true),
			KeyCode::Char('n') => self.submit = Some(false),
			KeyCode::Enter => self.submit = Some(self.selected == 0),

			_ => return false,
		}
		true
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let [area] = ctx.area.layout(&self.layout);

		let text_width = area
			.width
			.saturating_sub(2) // Border
			.saturating_sub(self.style.padding[1]) // Right padding
			.saturating_sub(self.style.padding[3]); // Left padding
		let text_height = self.measured_height(
			text_width,
			frame
				.area()
				.height
				.saturating_sub(4) // Border + Spacing + Button
				.saturating_sub(self.style.padding[0]) // Top padding
				.saturating_sub(self.style.padding[2]), // Bottom padding
		);
		let content_height = text_height + 4 + self.style.padding[0] + self.style.padding[2];
		let [area] =
			area.layout(&Layout::vertical([Constraint::Length(content_height)]).flex(Flex::Center));

		frame.render_widget(Clear, area);
		let block = self.style.block.clone().title(self.title.clone());
		let inner = block.inner(area);
		frame.render_widget(block, area);

		let paragraph_area = Rect {
			x: inner.x + self.style.padding[3], // Left padding
			y: inner.y + self.style.padding[0], // Top padding
			width: inner
				.width
				.saturating_sub(self.style.padding[1]) // Right padding
				.saturating_sub(self.style.padding[3]), // Left padding
			height: inner
				.height
				.saturating_sub(self.style.padding[0]) // Top padding
				.saturating_sub(self.style.padding[2]) // Bottom padding
				.saturating_sub(2), // Empty line + Button
		};
		frame.render_widget(&self.content, paragraph_area);

		let style_yes = self.style.buttons[(self.selected == 0) as usize];
		let style_no = self.style.buttons[(self.selected == 1) as usize];
		let buttons = Line::from(vec![
			Span::styled("Y", style_yes.underlined()),
			Span::styled("es", style_yes),
			" ".repeat(self.style.spacing as usize).into(),
			Span::styled("N", style_no.underlined()),
			Span::styled("o", style_no),
		]);

		let button_width = buttons.width() as u16;
		let button_area = Rect {
			x: (paragraph_area.x + paragraph_area.width / 2).saturating_sub(button_width / 2),
			y: paragraph_area.y + paragraph_area.height + 1,
			width: button_width,
			height: 1,
		};
		frame.render_widget(&buttons, button_area);
	}

	fn height(&self) -> u16 {
		3
	}
}
