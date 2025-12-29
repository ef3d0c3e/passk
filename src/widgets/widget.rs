use core::panic;

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::Frame;

/// Overlay for Z-level support
#[derive(PartialEq, Eq)]
pub struct Overlay {
	pub z_level: u16,
	pub buffer: Buffer,
}

impl PartialOrd for Overlay {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.z_level.partial_cmp(&other.z_level)
	}
}

/// Render context for [`Component`]
pub struct ComponentRenderCtx<'c> {
	pub area: Rect,
	pub selected: bool,
	pub queue: &'c mut Vec<Overlay>,
	pub depth: usize,
	pub cursor: Option<(usize, Position)>,
}

impl<'c> ComponentRenderCtx<'c> {
	pub fn push(&mut self, overlay: Overlay) {
		let idx = self.queue.partition_point(|o| o.z_level <= overlay.z_level);
		self.queue.insert(idx, overlay);
	}

	pub fn with_child<R, F>(&mut self, f: F) -> R
	where
		F: FnOnce(&mut Self) -> R,
	{
		self.depth += 1;
		if let Some(cursor) = self.cursor {
			if cursor.0 < self.depth {
				self.cursor = None
			}
		}
		let ret = f(self);
		self.depth -= 1;
		ret
	}

	pub fn set_cursor(&mut self, pos: Position) {
		if let Some(cursor) = &self.cursor {
			if cursor.0 > self.depth {
				return;
			}
		}
		self.cursor = Some((self.depth, pos));
	}
}

pub trait Component {
	/// Send inputs to the component
	/// Return `true` if the input was processed, `false` otherwise
	fn input(&mut self, key: &KeyEvent) -> bool;
	/// Render the component
	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx);
	/// Widget height, for vertical layouts
	fn height(&self) -> u16;
}
