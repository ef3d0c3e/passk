use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::widgets::checkbox::Checkbox;
use crate::widgets::combo_box::ComboBox;
use crate::widgets::text_input::TextInput;

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
}

impl<'c> ComponentRenderCtx<'c> {
	pub fn push(&mut self, overlay: Overlay) {
		let idx = self.queue.partition_point(|o| o.z_level <= overlay.z_level);
		self.queue.insert(idx, overlay);
	}
}

pub trait Component {
	/// Send inputs to the component
	fn input(&mut self, key: &KeyEvent) -> bool;
	/// Render the component
	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx);
	/// Widget height, for vertical layouts
	fn height(&self) -> u16;

	fn accept(&self, visitor: &mut dyn ComponentVisitor);
}

pub trait ComponentVisitor {
	#[allow(unused)]
	fn visit_checkbox(&mut self, checkbox: &Checkbox) {}
	#[allow(unused)]
	fn visit_text_input(&mut self, text_input: &TextInput) {}
	#[allow(unused)]
	fn visit_combo_box(&mut self, combo_box: &ComboBox) {}
}
