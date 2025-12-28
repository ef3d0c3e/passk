use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Clear;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use ratatui::Frame;

use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

pub enum FormEvent<'s> {
	Focus {
		previous: Option<usize>,
		current: Option<usize>,
	},
	Edit {
		id: usize,
		key: &'s KeyEvent,
	},
	Key {
		key: &'s KeyEvent,
	},
}

#[derive(Debug)]
pub enum FormSignal<T> {
	Exit,
	Return(T),
}

pub struct FormStyle {
	pub bg: Color,
}

pub trait Form {
	type Return;

	// Box<..> could become &..
	fn component_count(&self) -> usize;
	fn component(&self, index: usize) -> Option<&dyn Component>;
	fn component_mut(&mut self, index: usize) -> Option<&mut dyn Component>;

	fn selected(&self) -> Option<usize>;
	fn set_selected(&mut self, selected: Option<usize>);

	fn get_style(&self) -> &FormStyle;

	fn scroll(&self) -> u16;
	fn set_scroll(&self, scroll: u16);

	fn event(&mut self, ev: FormEvent) -> Option<FormSignal<Self::Return>>;

	fn render_form(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx);
}

// Focus needs to update scrolling. Scrolling must be based on the focus widget's height
pub trait FormExt: Form {
	fn ensure_visible(&self, viewport_height: u16) {
		let Some(selected) = self.selected() else {
			return;
		};

		let y: u16 = (0..selected).map(|i| self.component(i).unwrap())
			.map(|c| c.height())
			.sum();
		let h = self.component(selected).unwrap().height();
		let scroll = self.scroll();

		if y < scroll {
			self.set_scroll(y);
		} else if y + h > scroll + viewport_height {
			self.set_scroll(y + h - viewport_height);
		}
	}

	fn focus_next(&mut self) {
		match (self.selected(), self.component_count() == 0) {
			(_, true) => self.set_selected(None),
			(None, false) => self.set_selected(Some(0)),
			(Some(x), false) => {
				if self.component_count() > x + 1 {
					self.set_selected(Some(x + 1));
				} else {
					self.set_selected(Some(x));
				}
			}
		}
	}

	fn focus_prev(&mut self) {
		match (self.selected(), self.component_count() == 0) {
			(_, true) => self.set_selected(None),
			(None, false) => self.set_selected(None),
			(Some(x), false) => {
				if x > 0 {
					self.set_selected(Some(x - 1));
				} else {
					self.set_selected(Some(x));
				}
			}
		}
	}

	fn input(&mut self, key: &KeyEvent) -> Option<FormSignal<<Self as Form>::Return>> {
		if let Some(selected) = self.selected() {
			let eaten = self.component_mut(selected).unwrap().input(key);
			if let Some(signal) = self.event(FormEvent::Edit { id: selected, key }) {
				return Some(signal);
			}
			if eaten {
				return None;
			}
		}

		match key.code {
			KeyCode::Tab | KeyCode::Down => {
				let previous = self.selected();
				self.focus_next();
				self.event(FormEvent::Focus {
					previous,
					current: self.selected(),
				});
			}
			KeyCode::BackTab | KeyCode::Up => {
				let previous = self.selected();
				self.focus_prev();
				self.event(FormEvent::Focus {
					previous,
					current: self.selected(),
				});
			}
			_ => return self.event(FormEvent::Key { key }),
		}
		None
	}

	/// Render the form body
	fn render_body(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		frame.render_widget(Clear, ctx.area);

		// Final render rectangle
		let inner_area = Rect {
			x: ctx.area.x,
			y: ctx.area.y,
			width: ctx.area.width.saturating_sub(2), // -2 for scrollbar
			height: ctx.area.height,
		};

		// Fill with default color
		let bg = Style::default().bg(self.get_style().bg);
		for y in ctx.area.top()..ctx.area.bottom() {
			for x in ctx.area.left()..ctx.area.right() {
				frame.buffer_mut()[(x, y)].set_symbol(" ").set_style(bg);
			}
		}

		self.ensure_visible(inner_area.height);
		let mut queue = vec![];

		let mut y = inner_area.y.saturating_sub(self.scroll());
		for (idx, component) in (0..self.component_count()).map(|i| (i, self.component(i).unwrap())) {
			let h = component.height();
			let rect = Rect {
				x: inner_area.x,
				y,
				width: inner_area.width,
				height: h,
			};

			// Only render if visible
			if rect.y + rect.height > inner_area.y && rect.y < inner_area.y + inner_area.height {
				let mut ctx = ComponentRenderCtx {
					area: rect,
					selected: Some(idx) == self.selected(),
					queue: &mut queue,
				};
				component.render(frame, &mut ctx);
			}

			y += h;
		}

		let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalLeft);
		let max_scroll = y.saturating_sub(inner_area.height);
		let mut scroll_state =
			ScrollbarState::new(y as usize).position(self.scroll().min(max_scroll) as usize);
		let scrollbar_area = Rect {
			x: ctx.area.x + ctx.area.width.saturating_sub(1),
			y: ctx.area.y,
			width: 1,
			height: ctx.area.height,
		};
		frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scroll_state);

		// Render queue
		let buffer = frame.buffer_mut();
		for overlay in queue {
			buffer.merge(&overlay.buffer);
		}
	}
}

impl<T: Form + ?Sized> FormExt for T {}

impl<T: FormExt + ?Sized> Component for T {
	fn input(&mut self, key: &KeyEvent) -> bool {
		let _ = FormExt::input(self, key);
		false
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		self.render_form(frame, ctx);
	}

	fn height(&self) -> u16 {
		(0..self.component_count())
			.map(|i| self.component(i).unwrap().height())
			.sum::<u16>()
	}
}
