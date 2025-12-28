use std::sync::LazyLock;

use crossterm::event::KeyEvent;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentVisitor;

use super::widget::ComponentRenderCtx;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LabelDisplay<'s> {
	/// [Label][spacing][Widget]
	Inline {
		spacing: u16,
	},
	/// [Label]
	/// [Widget]
	#[default]
	Newline,

	Block {
		block: Block<'s>,
	},
}

#[derive(Debug, Clone, Default)]
pub struct LabelStyle<'s> {
	pub padding: [u16; 2],
	pub display: LabelDisplay<'s>,
	pub style: Option<Style>,
	pub style_selected: Option<Style>,
}

impl LabelStyle<'_> {
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

static DEFAULT_STYLE: LazyLock<LabelStyle> = LazyLock::new(LabelStyle::default);

pub struct Labeled<'s, T>
where
	T: Component,
{
	label: Span<'s>,
	style: &'s LabelStyle<'s>,
	pub inner: T,
}

impl<'s, T> Labeled<'s, T>
where
	T: Component,
{
	pub fn new(label: Span<'s>, inner: T) -> Self {
		Self {
			label,
			style: &DEFAULT_STYLE,
			inner,
		}
	}

	pub fn style(mut self, style: &'s LabelStyle) -> Self {
		self.style = style;
		self
	}
}

impl<T> Component for Labeled<'_, T>
where
	T: Component,
{
	fn input(&mut self, key: &KeyEvent) -> bool {
		self.inner.input(key)
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		match &self.style.display {
			LabelDisplay::Inline { spacing } => {
				let width = self.label.width();
				let offset = spacing + width as u16;

				frame.render_widget(
					self.label.clone().style(if ctx.selected {
						self.style.style_selected()
					} else {
						self.style.style()
					}),
					ctx.area,
				);

				ctx.area.x += offset;
				ctx.area.width = ctx.area.width.saturating_sub(offset);
			}
			LabelDisplay::Newline => {
				frame.render_widget(
					self.label.clone().style(if ctx.selected {
						self.style.style_selected()
					} else {
						self.style.style()
					}),
					ctx.area,
				);

				ctx.area.y += 1;
				ctx.area.height = ctx.area.height.saturating_sub(1);
			}
			LabelDisplay::Block { block } => {
				let block = block
					.clone()
					.title(self.label.clone())
					.style(if ctx.selected {
						self.style.style_selected()
					} else {
						self.style.style()
					});
				frame.render_widget(block, ctx.area);
				ctx.area.x += 1;
				ctx.area.y += 1;
				ctx.area.width = ctx.area.width.saturating_sub(2);
				ctx.area.height = ctx.area.height.saturating_sub(2);
			}
		}
		self.inner.render(frame, ctx);
	}

	fn height(&self) -> u16 {
		self.inner.height()
			+ match self.style.display {
				LabelDisplay::Inline { spacing: _ } => 0,
				LabelDisplay::Newline => 1,
				LabelDisplay::Block { block: _ } => 2,
			}
	}
}
