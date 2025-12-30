use std::path::Path;

use crate::data::database::Database;

pub static MAGIC: &'static [u8] = b"\x89PASSK";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PasskVersion {
	#[default]
	V0_1,
}

impl Into<&'static str> for PasskVersion {
	fn into(self) -> &'static str {
		match self {
			PasskVersion::V0_1 => "0.1",
		}
	}
}

impl TryFrom<&[u8]> for PasskVersion {
	type Error = &'static str;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		match value {
			b"0.1" => Ok(PasskVersion::V0_1),
			_ => Err("Unknown version"),
		}
	}
}

pub fn load_database(path: &Path) -> Result<Database, String> {
	let bytes =
		std::fs::read(path).map_err(|err| format!("Failed to read '{}': {err}", path.display()))?;
	if !bytes.starts_with(MAGIC) {
		return Err(format!(
			"Failed to verify MAGIC number in '{}'",
			path.display()
		));
	}
	let nl = bytes
		.iter()
		.rposition(|c| *c == b'\n')
		.ok_or(format!("Invalid header in '{}'", path.display()))?;
	let _ = PasskVersion::try_from(&bytes[MAGIC.len()..nl])
		.map_err(|err| format!("Invaid header in '{}': {err}", path.display()))?;
	let payload = &bytes[nl + 1..];
	// TODO: Migrate DB structure
	let db: Database = serde_json::from_slice(payload)
		.map_err(|err| format!("Failed to deserialize '{}': {err}", path.display()))?;
	Ok(db)
}
