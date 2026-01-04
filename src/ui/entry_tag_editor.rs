use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::symbols::border::QUADRANT_OUTSIDE;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::Frame;

use crate::data::entry::EntryTag;
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

pub struct EntryTagEditor {
	style: FormStyle,
	title: String,
	input: Labeled<'static, TextInput<'static>>,
}

impl EntryTagEditor {
	pub fn new(title: String, tags: &Vec<EntryTag>) -> Self {
		let input = tags
			.iter()
			.map(|x| x.name.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		Self {
			style: FormStyle {
				border: true,
				bg: Color::from_u32(0x2f2f2f),
			},
			title,
			input: Labeled::new("Tags".into(), TextInput::new().with_input(input).style(&TEXTINPUT_STYLE)).style(&LABEL_STYLE),
		}
	}

	pub fn submit(&self) -> Option<Vec<EntryTag>> {
		let mut result = vec![];
		let mut rest = &self.input.inner.get_input()[..];
		// TODO: Create a global tag registry to source icons/colors froms
		loop {
			if let Some(next) = rest.find(',') {
				rest.trim();
				result.push(EntryTag {
					name: rest[..next].to_string(),
					icon: None,
					color: None,
				});
				rest = &rest[next + 1..];
			} else {
				rest.trim();
				result.push(EntryTag {
					name: rest.to_string(),
					icon: None,
					color: None,
				});
				break;
			}
		}
		Some(result)
	}
}

impl Form for EntryTagEditor {
	fn component_count(&self) -> usize {
		1
	}

	fn component(&self, index: usize) -> Option<&dyn Component> {
		(index == 0).then_some(&self.input)
	}

	fn component_mut(&mut self, index: usize) -> Option<&mut dyn Component> {
		(index == 0).then_some(&mut self.input)
	}

	fn selected(&self) -> Option<usize> {
		Some(0)
	}

	fn set_selected(&mut self, _selected: Option<usize>) {}

	fn get_style(&self) -> &FormStyle {
		&self.style
	}

	fn scroll(&self) -> u16 {
		0
	}

	fn set_scroll(&self, _scroll: u16) {}

	fn input_form(&mut self, key: &KeyEvent) -> Option<FormSignal> {
		if FormExt::input(self, key) {
			return None;
		}
		match key.code {
			KeyCode::Enter => Some(FormSignal::Return),
			KeyCode::Esc => Some(FormSignal::Exit),
			_ => None,
		}
	}

	fn render_form(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let area = ctx.area;
		let border = Block::bordered()
			.border_set(QUADRANT_OUTSIDE)
			.title(self.title.clone())
			.title_style(Style::default().fg(Color::White))
			.title_alignment(ratatui::layout::HorizontalAlignment::Center)
			.bg(self.style.bg)
			.fg(Color::from_u32(0x1a1a1f));
		frame.render_widget(Clear, area);
		frame.render_widget(border, area);
		ctx.area.x += 1;
		ctx.area.width = ctx.area.width.saturating_sub(2);
		ctx.area.y += 1;
		ctx.area.height = ctx.area.height.saturating_sub(2);
		self.render_body(frame, ctx);
	}
}
