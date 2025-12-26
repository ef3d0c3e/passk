use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::MediaKeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use ratatui::Frame;

pub trait Component {
	/// Send inputs to the component
	fn input(&mut self, key: &KeyEvent);
	/// Render the component
	fn render(&self, frame: &mut Frame, ctx: &ComponentRenderCtx);
	/// Widget height, for vertical layouts
	fn height(&self) -> u16;
}

pub struct ComponentRenderCtx {
	pub area: Rect,
	pub selected: bool,
}

pub trait Form {
	fn components(&self) -> &[Box<dyn Component>];
	fn components_mut(&mut self) -> &mut [Box<dyn Component>];

	fn selected(&self) -> Option<usize>;
	fn set_selected(&mut self, selected: Option<usize>);

	fn scroll(&self) -> u16;
	fn set_scroll(&self, scroll: u16);
}

// Focus needs to update scrolling. Scrolling must be based on the focus widget's height
pub trait FormExt: Form {
	fn ensure_visible(&self, viewport_height: u16) {
		let Some(selected) = self.selected() else { return };

		let y: u16 = self.components()
			.iter()
			.take(selected)
			.map(|c| c.height())
			.sum();

		let h = self.components()[selected].height();
		let scroll = self.scroll();

		if y < scroll {
			self.set_scroll(y);
		} else if y + h > scroll + viewport_height {
			self.set_scroll(y + h - viewport_height);
		}
	}

	fn focus_next(&mut self) {
		match (self.selected(), self.components().is_empty()) {
			(_, true) => self.set_selected(None),
			(None, false) => self.set_selected(Some(0)),
			(Some(x), false) => {
				if self.components().len() > x + 1 {
					self.set_selected(Some(x + 1));
				} else {
					self.set_selected(Some(x));
				}
			}
		}
	}

	fn focus_prev(&mut self) {
		match (self.selected(), self.components().is_empty()) {
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
}

impl<T: FormExt + ?Sized> Component for T {
	fn input(&mut self, key: &KeyEvent) {
		if let Some(selected) = self.selected() {
			self.components_mut()[selected].input(key);
		}

		match key.code {
			KeyCode::Tab => self.focus_next(),
			KeyCode::BackTab => self.focus_prev(),
			_ => {}
		}
	}

	fn render(&self, frame: &mut Frame, ctx: &ComponentRenderCtx) {
		// Final render rectangle
		let inner_area = Rect {
			x: ctx.area.x,
			y: ctx.area.y,
			width: ctx.area.width.saturating_sub(2), // -2 for scrollbar
			height: ctx.area.height,
		};

		self.ensure_visible(inner_area.height);

		let mut y = inner_area.y.saturating_sub(self.scroll());
		for (idx, component) in self.components().iter().enumerate() {
			let h = component.height();
			let rect = Rect {
				x: inner_area.x,
				y,
				width: inner_area.width,
				height: h,
			};

			// Only render if visible
			if rect.y + rect.height > inner_area.y && rect.y < inner_area.y + inner_area.height {
				component.render(
					frame,
					&ComponentRenderCtx {
						area: rect,
						selected: Some(idx) == self.selected(),
					},
				);
			}

			y += h;
		}

		let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalLeft);
		let max_scroll = y.saturating_sub(inner_area.height);
		let mut scroll_state = ScrollbarState::new(y as usize)
			.position(self.scroll().min(max_scroll) as usize); 
		let scrollbar_area = Rect {
			x: ctx.area.x + ctx.area.width.saturating_sub(2),
			y: ctx.area.y,
			width: 1,
			height: ctx.area.height,
		};
		frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scroll_state);
	}

	fn height(&self) -> u16 {
		self.components()
			.iter()
			.fold(0, |r, component| r + component.height())
	}
}
