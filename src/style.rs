use ratatui::style::Color;

pub const HELP_LINE_BG: Color = Color::from_u32(0x1a60b5);
/// Background for the entry editor: Color1, Color2, Selected
pub const ENTRY_BG: [Color; 3] = [
	Color::from_u32(0x322b44),
	Color::from_u32(0x241f31),
	Color::from_u32(0x5d507f),
];
