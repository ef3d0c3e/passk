use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::Frame;

pub struct ConfirmDialog<'s> {
	title: Line<'s>,
	description: Vec<ListItem<'s>>,
	layout: [Layout; 2],
	selected: i32,
}

impl<'s> ConfirmDialog<'s> {
	pub fn new(title: Line<'s>, description: Vec<ListItem<'s>>) -> Self {
		let horizontal = Layout::horizontal([Constraint::Percentage(30)]).flex(Flex::Center);
		let vertical = Layout::vertical([Constraint::Length((4 + description.len()) as u16)])
			.flex(Flex::Center);
		Self {
			title,
			description,
			layout: [horizontal, vertical],
			selected: 0,
		}
	}

	pub fn input(&mut self, key: &KeyEvent) -> Option<bool> {
		match key.code {
			KeyCode::Esc | KeyCode::Char('n') => Some(false),
			KeyCode::Char('y') => Some(true),
			KeyCode::Enter => Some(self.selected == 0),
			KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
				self.selected = 0;
				None
			}
			KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
				self.selected = 1;
				None
			}
			_ => None,
		}
	}

	pub fn draw(&self, frame: &mut Frame) {
		let area = frame.area();
		let [area] = area.layout(&self.layout[0]);
		let [area] = area.layout(&self.layout[1]);

		// Yes/no buttons
		let padding = (area.width as usize - 2 - "yes  no".len()) / 2;
		let buttons = Line::from(vec![
			(0..padding).map(|_| ' ').collect::<String>().into(),
			"y".underlined()
				.fg(if self.selected == 0 {
					Color::Black
				} else {
					Color::White
				})
				.bg(if self.selected == 0 {
					Color::White
				} else {
					Color::default()
				}),
			"es".fg(if self.selected == 0 {
				Color::Black
			} else {
				Color::White
			})
			.bg(if self.selected == 0 {
				Color::White
			} else {
				Color::default()
			}),
			"  ".into(),
			"n".underlined()
				.fg(if self.selected == 1 {
					Color::Black
				} else {
					Color::White
				})
				.bg(if self.selected == 1 {
					Color::White
				} else {
					Color::default()
				}),
			"o".fg(if self.selected == 1 {
				Color::Black
			} else {
				Color::White
			})
			.bg(if self.selected == 1 {
				Color::White
			} else {
				Color::default()
			}),
		]);
		let mut list = self.description.clone();
		list.push(ListItem::new(Line::from(vec![" ".into()])));
		list.push(ListItem::new(buttons));
		let widget = List::new(list).block(Block::bordered().title(self.title.clone()));

		frame.render_widget(Clear, area);
		frame.render_widget(widget, area);
	}
}
