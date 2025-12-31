use ratatui::style::Style;
use ratatui::text::Span;

#[derive(Debug, Clone)]
pub struct CustomTextInputStyle<'s> {
	/// |<padding0><marker0>Input<marker1><padding1>|
	pub padding: [u16; 2],
	pub markers: [Span<'s>; 2],
	/// Style override
	pub style: Option<Style>,
	/// Selected style override
	pub style_selected: Option<Style>,
}

pub trait CustomTextFormatter {
	
}

pub struct CustomTextInput<'s> {
	style: &'s CustomTextInputStyle<'s>,

	input: String,
	grapheme_count: usize,
	grapheme_index: usize,
	cursor_x: u16,
}
