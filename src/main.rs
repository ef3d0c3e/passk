use core::panic;
use std::cell::OnceCell;
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;

use clipboard_rs::ClipboardContext;
use color_eyre::eyre;
use color_eyre::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::{self};
use getopts::Options;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::DefaultTerminal;
use ratatui::Frame;

use crate::data::database::decrypt_database;
use crate::data::database::encrypt_database;
use crate::data::database::CipherData;
use crate::data::database::Data;
use crate::data::database::Database;
use crate::data::database::KdfData;
use crate::data::file::load_database;
use crate::data::file::save_database;
use crate::data::file::PasskVersion;
use crate::ui::explorer::Explorer;
use crate::ui::password::PasswordPrompt;
use crate::widgets::form::Form;
use crate::widgets::form::FormSignal;
use crate::widgets::popup::Popup;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

pub mod data;
pub mod style;
pub mod ui;
pub mod widgets;

pub static CLIPBOARD_CTX: LazyLock<ClipboardContext> =
	LazyLock::new(|| ClipboardContext::new().unwrap());

struct App {
	db: Database,
	path: PathBuf,
	password: OnceCell<String>,
	data: OnceCell<Data>,
	explorer: OnceCell<Explorer>,
	password_prompt: Option<PasswordPrompt>,

	message: Option<Popup<'static>>,
}

impl App {
	fn error(&mut self, message: String) {
		self.message = Some(Popup::new(
			"Error".into(),
			Paragraph::new(Text::from(message)),
		));
	}

	fn get_data(&mut self) -> (String, Data, Database) {
		let password = self.password.get().cloned().unwrap();
		let mut data = self.data.get().cloned().unwrap();
		data.entries = self
			.explorer
			.get()
			.map(|explorer| explorer.submit())
			.unwrap();
		let db = self.db.clone();

		(password, data, db)
	}

	pub fn new(name: String, path: PathBuf) -> Result<Self, String> {
		let (db, new) = if !path.exists() {
			let mut salt = [0u8; 16];
			rand::fill(&mut salt);
			(
				Database {
					version: data::database::Version::V1,
					cipher: CipherData::XChaCha20Poly1305V1 {},
					kdf: KdfData::Argon2Id {
						salt,
						memory: 65536,
						iterations: 2,
						key_len: CipherData::XChaCha20Poly1305V1 {}.key_len() as u16,
						parallelism: 2,
					},
					blob: vec![],
				},
				true,
			)
		} else {
			(load_database(&path)?, false)
		};
		Ok(Self {
			db,
			path,
			password: OnceCell::default(),
			data: OnceCell::default(),
			explorer: OnceCell::default(),
			password_prompt: Some(PasswordPrompt::new(name, new)),
			message: None,
		})
	}

	fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
		loop {
			terminal.draw(|frame| self.draw(frame))?;

			if let Event::Key(key) = event::read()? {
				// Password prompt
				if let Some(password) = &mut self.password_prompt {
					match password.input_form(&key) {
						Some(FormSignal::Return) => {}
						Some(FormSignal::Exit) => return Ok(()),
						_ => continue,
					}
					let Some(pwd) = password.submit() else {
						return Ok(());
					};
					let mut data = if password.is_new() {
						// Create default data
						Data::default()
					} else {
						// Decrypt data
						match decrypt_database(&self.db, pwd.as_str()) {
							Ok(data) => data,
							Err(err) => {
								password.set_error(
									"Invalid Password".into(),
									format!("Failed to decrypt database: {err}"),
								);
								continue;
							}
						}
					};
					self.password.set(pwd).unwrap();
					self.explorer
						.set(Explorer::new(std::mem::take(&mut data.entries)))
						.map_err(|_| ())
						.unwrap();
					self.data.set(data).unwrap();
					self.password_prompt = None;
					continue;
				}
				// Message
				if let Some(message) = &mut self.message {
					if !message.input(&key) {
						self.message = None;
					}
					continue;
				}
				// Explorer
				if let Some(explorer) = self.explorer.get_mut() {
					if explorer.input(&key) {
						continue;
					}
				}

				if let KeyCode::Char('q') = key.code {
					let (password, data, mut db) = self.get_data();
					db.blob = match encrypt_database(&data, &self.db, &password) {
						Ok(blob) => blob,
						Err(err) => {
							self.error(format!("Failed to encrypt database: {err}"));
							continue;
						}
					};
					if let Err(err) = save_database(&db, &self.path) {
						self.error(format!("Failed to save database: {err}"));
						continue;
					}
					return Ok(());
				}
			}
		}
	}

	fn draw(&self, frame: &mut Frame) {
		let mut overlays = vec![];
		let mut ctx = ComponentRenderCtx {
			area: frame.area(),
			selected: false,
			queue: &mut overlays,
			depth: 0,
			cursor: None,
		};
		// Password prompt
		if let Some(password) = &self.password_prompt {
			ctx.selected = true;
			password.render_form(frame, &mut ctx);
		}
		// Message
		else if let Some(message) = &self.message {
			ctx.selected = true;
			message.render(frame, &mut ctx);
		}
		// Explorer
		else if let Some(explorer) = self.explorer.get() {
			explorer.render(frame, &mut ctx);
		}

		if let Some((_, cursor)) = ctx.cursor {
			frame.set_cursor_position(cursor);
		}
	}
}

fn print_usage(program: &str, opts: Options) {
	let brief = format!("Usage: {program} [OPTIONS] path/to/database.pk");
	print!("{}", opts.usage(&brief));
}

fn print_version() {
	print!(
		"PassK -- A simple password manager
Copyright (c) 2026 ef3d0c3e
PassK is licensed under the GNU General Public License version 3 (GPLv3),
under the terms of the Free Software Foundation <https://www.gnu.org/licenses/gpl-3.0.en.html>.

This program is free software; you may modify and redistribute it.
There is NO WARRANTY, to the extent permitted by law.

PassK version: {}\n",
		<&'static str>::from(PasskVersion::default())
	)
}

fn main() -> Result<()> {
	let args: Vec<String> = env::args().collect();
	let program = &args[0];

	let mut opts = Options::new();
	opts.optflag("h", "help", "Display help menu");
	opts.optflag("v", "version", "Display program version");

	let m = match opts.parse(&args[1..]) {
		Ok(m) => m,
		Err(err) => {
			panic!("{err}");
		}
	};
	if m.opt_present("h") {
		print_usage(&program, opts);
		return Ok(());
	}
	if m.opt_present("v") {
		print_version();
		return Ok(());
	}
	if m.free.len() != 1
	{
		eprintln!("Expected path to a database file");
		return Ok(())
	}

	let path = PathBuf::from(&m.free[0]);

	let name = &args[1][args[1].rfind('/').unwrap_or(0)..];

	let terminal = ratatui::init();
	let app_result = App::new(name.into(), path)
		.map_err(|err| eyre::eyre!(err))?
		.run(terminal);
	ratatui::restore();
	app_result
}
