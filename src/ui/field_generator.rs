use std::cell::RefCell;
use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use rand::Rng;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::symbols::border::QUADRANT_OUTSIDE;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::Frame;

use crate::widgets::combo_box::ComboBox;
use crate::widgets::combo_box::ComboBoxStyle;
use crate::widgets::combo_box::ComboItem;
use crate::widgets::form::Form;
use crate::widgets::form::FormExt;
use crate::widgets::form::FormSignal;
use crate::widgets::form::FormStyle;
use crate::widgets::label::LabelDisplay;
use crate::widgets::label::LabelStyle;
use crate::widgets::label::Labeled;
use crate::widgets::text_input::TextInput;
use crate::widgets::text_input::TextInputStyle;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

static CHARSET_TYPE: LazyLock<[ComboItem; 4]> = LazyLock::new(|| {
	[
		ComboItem {
			kind: "ASCII".into(),
			icon: "󱅈 ".into(),
			value: "Alphanumeric".into(),
		},
		ComboItem {
			kind: "ASCII".into(),
			icon: "󱅈 ".into(),
			value: "Alphabet".into(),
		},
		ComboItem {
			kind: "ASCII".into(),
			icon: "󰟵 ".into(),
			value: "Base86".into(),
		},
		ComboItem {
			kind: "Unicode".into(),
			icon: "󰟵 ".into(),
			value: "Custom".into(),
		},
	]
});

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CharsetKind {
	#[default]
	Alphanum,
	Alpha,
	Base86,
	Custom,
}

impl TryFrom<usize> for CharsetKind {
	type Error = &'static str;

	fn try_from(value: usize) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(CharsetKind::Alphanum),
			1 => Ok(CharsetKind::Alpha),
			2 => Ok(CharsetKind::Base86),
			3 => Ok(CharsetKind::Custom),
			_ => Err("Invalid value"),
		}
	}
}

impl CharsetKind {
	fn name(&self) -> &'static str {
		match self {
			CharsetKind::Alphanum => "Alphanumeric",
			CharsetKind::Alpha => "Alphabet",
			CharsetKind::Base86 => "Base86",
			CharsetKind::Custom => "Custom",
		}
	}
}

static LABEL_STYLE: LazyLock<LabelStyle> = LazyLock::new(|| LabelStyle {
	padding: [0, 0],
	display: LabelDisplay::Block {
		block: Box::new(Block::bordered()),
	},
	style: Some(Style::default().fg(Color::White)),
	style_selected: None,
});
static TEXTINPUT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(|| TextInputStyle {
	padding: [0, 0],
	markers: ["".into(), "".into()],
	style: Some(Style::default().fg(Color::White)),
	style_selected: None,
});
static COMBOBOX_STYLE: LazyLock<ComboBoxStyle> = LazyLock::new(|| ComboBoxStyle {
	padding: Default::default(),
	markers: ["".into(), "".into()],
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
});

pub struct FieldGenerator {
	title: String,
	style: FormStyle,

	prev_charset_type: Option<CharsetKind>,
	charset_type: Option<CharsetKind>,
	field_len: Labeled<'static, TextInput<'static>>,
	field_charset: Labeled<'static, ComboBox<'static, 'static>>,
	field_charset_custom: Option<Labeled<'static, TextInput<'static>>>,

	selected: Option<usize>,
	scroll: RefCell<u16>,
}

impl FieldGenerator {
	pub fn new(title: String) -> Self {
		Self {
			title,
			style: FormStyle {
				bg: Color::from_u32(0x2f2f2f),
				border: true,
			},
			prev_charset_type: None,
			charset_type: Some(CharsetKind::Alphanum),
			field_len: Labeled::new("Length".into(), TextInput::new().style(&TEXTINPUT_STYLE))
				.style(&LABEL_STYLE),
			field_charset: Labeled::new(
				"Charset".into(),
				ComboBox::new(CHARSET_TYPE.as_slice())
					.style(&COMBOBOX_STYLE)
					.with_input(CharsetKind::Alphanum.name().into()),
			)
			.style(&LABEL_STYLE),
			field_charset_custom: None,
			selected: None,
			scroll: RefCell::default(),
		}
	}

	pub fn submit(&self) -> Option<String> {
		let charset_kind = self.charset_type?;
		let length = self.field_len.inner.submit().parse::<usize>().ok()?;
		let charset: Vec<char> = match charset_kind {
    CharsetKind::Alphanum => "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect(),
    CharsetKind::Alpha=> "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect(),
    CharsetKind::Base86 => "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuz".chars().collect(),
    CharsetKind::Custom => {
		let custom = self.field_charset_custom.as_ref()?;
		custom.inner.submit().chars().collect()
	},
		};
		if charset.is_empty() || length == 0 {
			return None;
		}
		let mut rng = rand::rng();
		let random = (0..length)
			.map(|_| charset[rng.random_range(0..charset.len())])
			.collect::<String>();
		Some(random)
	}
}

impl Form for FieldGenerator {
	fn component_count(&self) -> usize {
		match self.charset_type {
			Some(CharsetKind::Custom) => 3,
			_ => 2,
		}
	}

	fn component(&self, index: usize) -> Option<&dyn Component> {
		match index {
			0 => Some(&self.field_len),
			1 => Some(&self.field_charset),
			2 => {
				if let Some(field) = &self.field_charset_custom {
					Some(field)
				} else {
					None
				}
			}
			_ => None,
		}
	}

	fn component_mut(&mut self, index: usize) -> Option<&mut dyn Component> {
		match index {
			0 => Some(&mut self.field_len),
			1 => Some(&mut self.field_charset),
			2 => {
				if let Some(field) = &mut self.field_charset_custom {
					Some(field)
				} else {
					None
				}
			}
			_ => None,
		}
	}

	fn selected(&self) -> Option<usize> {
		self.selected
	}

	fn set_selected(&mut self, selected: Option<usize>) {
		self.selected = selected
	}

	fn get_style(&self) -> &FormStyle {
		&self.style
	}

	fn scroll(&self) -> u16 {
		*self.scroll.borrow()
	}

	fn set_scroll(&self, scroll: u16) {
		*self.scroll.borrow_mut() = scroll;
	}

	fn input_form(&mut self, key: &KeyEvent) -> Option<FormSignal> {
		// Dispatch input to components
		if FormExt::input(self, key) {
			// Update state
			if self.selected == Some(1) {
				if let Some(Ok(kind)) = self.field_charset.inner.submit().map(CharsetKind::try_from)
				{
					if Some(kind) != self.prev_charset_type {
						self.prev_charset_type = self.charset_type;
						self.charset_type = Some(kind);
						self.field_charset_custom = None;
						if kind == CharsetKind::Custom {
							self.field_charset_custom = Some(
								Labeled::new(
									kind.name().into(),
									TextInput::new().style(&TEXTINPUT_STYLE),
								)
								.style(&LABEL_STYLE),
							);
						}
					}
				} else {
					self.prev_charset_type = self.charset_type;
					self.charset_type = None;
					self.field_charset_custom = None;
				}
			}
			return None;
		}

		// Quit
		if key.code == KeyCode::Esc {
			return Some(FormSignal::Exit);
		} else if key.code == KeyCode::Enter {
			return Some(FormSignal::Return);
		}

		None
	}

	fn render_form(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let area = ctx.area;
		let border = Block::bordered()
			.border_set(QUADRANT_OUTSIDE)
			.title(self.title.as_str())
			.title_style(Style::default().fg(Color::White))
			.title_alignment(ratatui::layout::HorizontalAlignment::Center)
			.bg(self.style.bg)
			.fg(Color::from_u32(0x1a1a1f));
		frame.render_widget(Clear, area);
		frame.render_widget(border, area);
		ctx.area.x += 1;
		ctx.area.width = ctx.area.width.saturating_sub(2);
		ctx.area.y += 2;
		ctx.area.height = ctx.area.height.saturating_sub(3);

		let entropy_area = Rect {
			x: ctx.area.x,
			y: (ctx.area.y + ctx.area.height).saturating_sub(1),
			width: ctx.area.width,
			height: 1,
		};
		let length = self.field_len.inner.submit().parse::<usize>().unwrap_or(0);
		let ent_value = match self.charset_type {
			Some(CharsetKind::Alphanum) => (length as f64) * 62f64.log2(),
			Some(CharsetKind::Alpha) => (length as f64) * 52f64.log2(),
			Some(CharsetKind::Base86) => (length as f64) * 86f64.log2(),
			Some(CharsetKind::Custom) => {
				let size = self
					.field_charset_custom
					.as_ref()
					.map(|f| f.inner.submit().chars().count())
					.unwrap_or(0);
				if size == 0 {
					0.0
				} else {
					(length as f64) * (size as f64).log2()
				}
			}
			None => 0.0,
		};
		let ent_style = Style::default().bold().fg(match ent_value as usize {
			0..64 => Color::Red,
			64..80 => Color::Yellow,
			80..90 => Color::LightGreen,
			_ => Color::Green,
		});
		let entropy = Line::from(vec![
			"Entropy".fg(Color::White).underlined(),
			": ".fg(Color::White),
			Span::from(format!("{ent_value}")).style(ent_style),
			Span::from("bits").style(ent_style),
		]);
		frame.render_widget(entropy, entropy_area);

		ctx.area.height = ctx.area.height.saturating_sub(1);
		self.render_body(frame, ctx);
	}
}
