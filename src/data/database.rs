use std::cell::Cell;

use argon2::Argon2;
use chacha20poly1305::KeyInit;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::data::entry::Entry;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Version {
	#[default]
	V1,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CipherData {
	XChaCha20Poly1305V1 {},
}

#[derive(Clone, Serialize, Deserialize)]
pub struct XChaCha20Poly1305BlobV1 {
	nonce: [u8; 24],
	// ciphertext || tag
	ciphertext: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum KdfData {
	Argon2Id {
		salt: [u8; 16],
		memory: u32,
		iterations: u32,
		key_len: u16,
		parallelism: u32,
	},
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Database {
	pub version: Version,
	pub cipher: CipherData,
	pub kdf: KdfData,

	// Cipher specific data
	pub blob: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
	pub iteration: u64,
	pub entries: Vec<Entry>,

	pub created_at: DateTime<Utc>,
	pub modified_at: DateTime<Utc>,
}

impl Default for Data {
	fn default() -> Self {
		Self {
			iteration: Default::default(),
			entries: Default::default(),
			created_at: Utc::now(),
			modified_at: Utc::now(),
		}
	}
}

fn derive_key(kdf: &KdfData, password: &str) -> Result<Vec<u8>, String> {
	match kdf {
		KdfData::Argon2Id {
			salt,
			memory,
			iterations,
			key_len,
			parallelism,
		} => {
			let config =
				argon2::Params::new(*memory, *iterations, *parallelism, Some(*key_len as usize))
					.map_err(|err| format!("Failed to build argon2 params: {err}"))?;

			let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, config);
			let mut key = vec![0u8; *key_len as usize];
			argon
				.hash_password_into(password.as_bytes(), salt, &mut key)
				.map_err(|err| format!("Failed to hash password: {err}"))?;
			Ok(key)
		}
	}
}

pub fn decrypt_database(db: &Database, password: &str) -> Result<Data, String> {
	let key = derive_key(&db.kdf, password)?;

	match &db.cipher {
		CipherData::XChaCha20Poly1305V1 {} => {
			let blob: XChaCha20Poly1305BlobV1 = bincode2::deserialize(&db.blob)
				.map_err(|err| format!("Failed to deserialize blob into cipher blob: {err}"))?;
			let cipher = chacha20poly1305::XChaCha20Poly1305::new_from_slice(&key)
				.map_err(|err| format!("Failed to initialize chacha20-poly1305 cipher: {err}"))?;
			let mut header = db.clone();
			header.blob = vec![]; // Use an empty blob for AAD
			let plaintext = chacha20poly1305::aead::Aead::decrypt(
				&cipher,
				&blob.nonce.into(),
				chacha20poly1305::aead::Payload {
					msg: &blob.ciphertext,
					aad: bincode2::serialize(&header)
						.map_err(|err| format!("Failed to serialize database: {err}"))?
						.as_slice(),
				},
			)
			.map_err(|err| format!("Failed to decrypt chacha20-poly1305 ciphertext: {err}"))?;
			let data: Data = bincode2::deserialize(&plaintext)
				.map_err(|err| format!("Failed to deserialize database: {err}"))?;
			Ok(data)
		}
	}
}

pub fn encrypt_database(data: &Data, db: &Database, password: &str) -> Result<Vec<u8>, String> {
	let key = derive_key(&db.kdf, password)?;
	println!("Key: {key:#?}");

	match &db.cipher {
		CipherData::XChaCha20Poly1305V1 {} => {
			let plaintext = bincode2::serialize(data)
				.map_err(|err| format!("Failed to serialize data: {err}"))?;
			let cipher = chacha20poly1305::XChaCha20Poly1305::new_from_slice(&key)
				.map_err(|err| format!("Failed to initialize chacha20-poly1305 cipher: {err}"))?;
			let nonce =
				<chacha20poly1305::XChaCha20Poly1305 as chacha20poly1305::AeadCore>::generate_nonce(
					&mut chacha20poly1305::aead::OsRng,
				);
			let mut header = db.clone();
			header.blob = vec![]; // Use an empty blob for AAD
			let ciphertext = chacha20poly1305::aead::Aead::encrypt(
				&cipher,
				&nonce,
				chacha20poly1305::aead::Payload {
					msg: &plaintext,
					aad: bincode2::serialize(&header)
						.map_err(|err| format!("Failed to serialize database: {err}"))?
						.as_slice(),
				},
			)
			.map_err(|err| format!("Failed to encrypt using chacha20-poly1305: {err}"))?;
			let blob = XChaCha20Poly1305BlobV1 {
				nonce: nonce.into(),
				ciphertext,
			};
			bincode2::serialize(&blob).map_err(|err| format!("Failed to serialize data: {err}"))
		}
	}
}
