use crate::data::field::Field;
use crate::widgets::text_input::TextInput;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use rand::distr::Alphanumeric;
use rand::distr::SampleString;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::layout::Offset;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(u8)]
enum ActiveField {
	#[default]
	None,
	Name,
	Value,
	Hidden,
}

pub struct FieldEditor<'s> {
	name: TextInput<'s>,
	value: TextInput<'s>,
	hidden: bool,

	title: Line<'s>,
	layout: [Layout; 2],

	active: ActiveField,
	generator: Option<TextInput<'s>>,
}

/*
/* Name   : [.....]
 * Value  : [.....]
 * Hidden : [x]
 */
impl<'s> FieldEditor<'s> {
	pub fn new(title: Line<'s>) -> Self {
		let vertical = Layout::vertical([Constraint::Length(3 * 2 + 2)]);
		let horizontal =
			Layout::horizontal([Constraint::Percentage(100)]).flex(Flex::Center);
		Self {
			name: TextInput::new(Line::from("Name"), Constraint::Percentage(100)),
			value: TextInput::new(Line::from("Value"), Constraint::Percentage(100)),
			hidden: false,
			title,
			layout: [horizontal, vertical],
			active: ActiveField::default(),
			generator: None,
		}
	}

	pub fn with_field(mut self, field: &Field) -> Self {
		self.name = self.name.with_input(field.name.clone());
		self.value = self.value.with_input(field.value.clone());
		self
	}

	pub fn set_active(&mut self, active: ActiveField) {
		match self.active {
			ActiveField::Name => self.name.set_active(false),
			ActiveField::Value => self.value.set_active(false),
			_ => {}
		}
		self.active = active;
		match self.active {
			ActiveField::Name => self.name.set_active(true),
			ActiveField::Value => self.value.set_active(true),
			_ => {}
		}
	}

	fn submit(&mut self) -> Field {
		todo!();
		//Field {
		//	name: self.name.submit(),
		//	value: self.value.submit(),
		//	hidden: self.hidden,
		//}
	}

	pub fn input(&mut self, key: &KeyEvent) -> Option<Option<Field>> {
		// Password generator
		if let Some(generator) = &mut self.generator {
			if key.code == KeyCode::Enter {
				if let Ok(length) = generator.submit().parse::<i32>() {
					if length > 0 {
						let generated =
							Alphanumeric.sample_string(&mut rand::rng(), length as usize);
						match self.active {
							ActiveField::Name => self.name.set_input(generated),
							ActiveField::Value => self.value.set_input(generated),
							_ => {}
						}
					}
				}
				self.generator = None;
				return None;
			}
			generator.input(key);
			return None;
		}

		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			KeyCode::Down | KeyCode::Tab => {
				let next = match self.active {
					ActiveField::None => ActiveField::Name,
					ActiveField::Name => ActiveField::Value,
					ActiveField::Value => ActiveField::Hidden,
					ActiveField::Hidden => ActiveField::Hidden,
				};
				self.set_active(next);
			}
			KeyCode::Up | KeyCode::BackTab => {
				let prev = match self.active {
					ActiveField::None => ActiveField::None,
					ActiveField::Name => ActiveField::Name,
					ActiveField::Value => ActiveField::Name,
					ActiveField::Hidden => ActiveField::Value,
				};
				self.set_active(prev);
			}
			KeyCode::Char(' ') if self.active == ActiveField::Hidden => self.hidden = !self.hidden,
			KeyCode::Char('g') if ctrl_pressed => match self.active {
				ActiveField::Name | ActiveField::Value => {
					let mut generator =
						TextInput::new(Line::from("Length"), Constraint::Percentage(100))
							.with_input("64".into());
					generator.set_active(true);
					self.generator = Some(generator)
				}
				_ => {}
			},
			KeyCode::Enter => {
				let field = self.submit();
				if field.name.trim().is_empty() {
					return Some(None);
				}
				return Some(Some(field));
			}
			KeyCode::Esc => return Some(None),
			_ => match self.active {
				ActiveField::Name => self.name.input(key),
				ActiveField::Value => self.value.input(key),
				_ => {}
			},
		}
		return None;
	}

	pub fn draw(&self, frame: &mut Frame, rect: Rect) {
		// Field generator
		if let Some(generator) = &self.generator {
			let mut area = rect;
			area.height = 3;
			generator.draw(frame, area);
			return;
		}

		let boxed = Block::bordered().title(self.title.clone());
		frame.render_widget(boxed, rect);

		let mut area = rect;
		area.width -= 2;
		area.height -= 2;
		let [area] = area.layout(&self.layout[0]);
		let [area] = area.layout(&self.layout[1]);
		let text = Text::from(Line::from(vec![
			"⮁".bold().fg(Color::Green),
			" (navigate) ".into(),
			"S-⮁".bold().fg(Color::Green),
			" (order) ".into(),
			"esc".bold().fg(Color::Green),
			" (cancel) ".into(),
			"enter".bold().fg(Color::Green),
			" (submit) ".into(),
			"space".bold().fg(Color::Green),
			" (toggle) ".into(),
			"C-g".bold().fg(Color::Green),
			" (generate) ".into(),
		]));
		let help_message = Paragraph::new(text);
		frame.render_widget(help_message, area.offset(Offset::new(0, 1)));
		self.name.draw(frame, area.offset(Offset::new(0, 2)));
		self.value.draw(frame, area.offset(Offset::new(0, 5)));

		// Checkbox
		let fg = if self.active == ActiveField::Hidden {
			Color::Yellow
		} else {
			Color::Green
		};
		let checkbox = Text::from(Line::from(vec![
			format!("[{}]", if self.hidden { "x" } else { " " }).fg(fg),
			" Hidden".fg(fg),
		]));
		let area = rect;
		let vertical = Layout::vertical([Constraint::Length(1)]);
		let horizontal =
			Layout::horizontal([Constraint::Length(rect.width - 2)]).flex(Flex::Center);
		let [area] = area.layout(&vertical);
		let [area] = area.layout(&horizontal);
		frame.render_widget(checkbox, area.offset(Offset::new(0, 8)));
	}
}
*/
