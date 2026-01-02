use std::cell::OnceCell;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use clipboard_rs::ClipboardContext;
use color_eyre::eyre;
use color_eyre::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use ratatui::Frame;

use crate::data::database::decrypt_database;
use crate::data::database::CipherData;
use crate::data::database::Data;
use crate::data::database::Database;
use crate::data::database::KdfData;
use crate::data::database::encrypt_database;
use crate::data::file::load_database;
use crate::data::file::save_database;
use crate::ui::explorer::Explorer;
use crate::ui::password::PasswordPrompt;
use crate::widgets::form::Form;
use crate::widgets::form::FormSignal;
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
}

impl App {
	pub fn new(path: PathBuf) -> Result<Self, String> {
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
						key_len: 32,
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
			password_prompt: Some(PasswordPrompt::new("Name".into(), new)),
		})
	}

	fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
		loop {
			terminal.draw(|frame| self.draw(frame))?;

			if let Event::Key(key) = event::read()? {
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
						decrypt_database(&self.db, pwd.as_str()).map_err(|err| eyre::eyre!(err))?
					};
					self.password.set(pwd).unwrap();
					self.explorer
						.set(Explorer::new(std::mem::take(&mut data.entries)))
						.map_err(|_| ())
						.unwrap();
					self.data.set(data).unwrap();
					self.password_prompt = None;
					continue
				}
				if let Some(explorer) = self.explorer.get_mut() {
					if explorer.input(&key) {
						continue;
					}
				}

				match key.code {
					KeyCode::Char('q') => {
						let mut data = self.data.take().unwrap();
						data.entries = self.explorer.take().unwrap().submit();
						let password = self.password.take().unwrap();
						let mut db = self.db.clone();
						db.blob = encrypt_database(&data, &self.db, &password).unwrap();
						save_database(&db, &self.path).unwrap();
						return Ok(())
					},
					_ => {}
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
		if let Some(password) = &self.password_prompt {
			ctx.selected = true;
			password.render_form(frame, &mut ctx);
		} else if let Some(explorer) = self.explorer.get() {
			explorer.render(frame, &mut ctx);
		}

		if let Some((_, cursor)) = ctx.cursor {
			frame.set_cursor_position(cursor);
		}
	}
}

fn main() -> Result<()> {
	let args: Vec<String> = env::args().collect();
	let path = PathBuf::from(&args[1]);

	/*
	if !path.exists() {
		let mut data = Data::default();
		let password = run(&mut data, true).unwrap();

		let mut salt = [0u8; 16];
		rand::fill(&mut salt);
		let db = Database {
			version: data::database::Version::V1,
			cipher: CipherData::XChaCha20Poly1305V1 {},
			kdf: KdfData::Argon2Id {
				salt,
				memory: 65536,
				iterations: 2,
				key_len: 64,
				parallelism: 4,
			},
			blob: Vec::default(),
		};
		// Create DB
	}
	*/
	//let db = Database {
	//	version: Default::default(),
	//	cipher: CipherData::XChaCha20Poly1305 { nonce: [0; 24] },
	//	kdf: KdfData::Argon2Id {
	//		salt: [0; 16],
	//		memory: 65536,
	//		iterations: 3,
	//		paralellism: true,
	//	},
	//	blob: vec![5, 7, 6],
	//};
	//println!("{}", serde_json::to_string_pretty(&db).unwrap());
	//Ok(())
	let terminal = ratatui::init();
	let app_result = App::new(path)
		.map_err(|err| eyre::eyre!(err))?
		.run(terminal);
	ratatui::restore();
	app_result
}
