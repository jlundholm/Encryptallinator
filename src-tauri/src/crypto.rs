use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAGIC: &[u8; 4] = b"EAI1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PayloadKind {
    File,
    DirectoryArchive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Payload {
    pub kind: PayloadKind,
    pub original_name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("The encrypted file format is invalid or unsupported.")]
    InvalidFormat,
    #[error("Encryption failed.")]
    EncryptionFailed,
    #[error("Decryption failed. Check the password or confirm the file was created by Encryptallinator.")]
    DecryptionFailed,
    #[error("Unable to serialize the encrypted payload: {0}")]
    Serialize(#[from] Box<bincode::ErrorKind>),
    #[error("Password key derivation failed.")]
    KeyDerivation,
    #[error("The crypto parameters are invalid.")]
    InvalidParameters,
}

pub fn encrypt_payload(payload: &Payload, password: &str) -> Result<Vec<u8>, CryptoError> {
    let serialized_payload = bincode::serialize(payload)?;
    let mut salt = [0_u8; SALT_LEN];
    let mut nonce = [0_u8; NONCE_LEN];

    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::InvalidParameters)?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), serialized_payload.as_ref())
        .map_err(|_| CryptoError::EncryptionFailed)?;

    let mut encrypted = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    encrypted.extend_from_slice(MAGIC);
    encrypted.extend_from_slice(&salt);
    encrypted.extend_from_slice(&nonce);
    encrypted.extend_from_slice(&ciphertext);

    Ok(encrypted)
}

pub fn decrypt_payload(bytes: &[u8], password: &str) -> Result<Payload, CryptoError> {
    if bytes.len() <= MAGIC.len() + SALT_LEN + NONCE_LEN {
        return Err(CryptoError::InvalidFormat);
    }

    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(CryptoError::InvalidFormat);
    }

    let salt_start = MAGIC.len();
    let nonce_start = salt_start + SALT_LEN;
    let data_start = nonce_start + NONCE_LEN;

    let salt: [u8; SALT_LEN] = bytes[salt_start..nonce_start]
        .try_into()
        .map_err(|_| CryptoError::InvalidFormat)?;
    let nonce: [u8; NONCE_LEN] = bytes[nonce_start..data_start]
        .try_into()
        .map_err(|_| CryptoError::InvalidFormat)?;

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::InvalidParameters)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), &bytes[data_start..])
        .map_err(|_| CryptoError::DecryptionFailed)?;

    bincode::deserialize(&plaintext).map_err(CryptoError::Serialize)
}

fn derive_key(password: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; KEY_LEN], CryptoError> {
    let params =
        Params::new(65_536, 3, 1, Some(KEY_LEN)).map_err(|_| CryptoError::InvalidParameters)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0_u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| CryptoError::KeyDerivation)?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{decrypt_payload, encrypt_payload, Payload, PayloadKind};

    #[test]
    fn file_payload_round_trip_succeeds() {
        let payload = Payload {
            kind: PayloadKind::File,
            original_name: "secrets.txt".to_string(),
            data: b"top secret".to_vec(),
        };

        let encrypted = encrypt_payload(&payload, "correct horse battery staple").unwrap();
        let decrypted = decrypt_payload(&encrypted, "correct horse battery staple").unwrap();

        assert_eq!(payload, decrypted);
    }

    #[test]
    fn directory_payload_round_trip_succeeds() {
        let payload = Payload {
            kind: PayloadKind::DirectoryArchive,
            original_name: "vault".to_string(),
            data: vec![1, 2, 3, 4, 5],
        };

        let encrypted = encrypt_payload(&payload, "folder-password").unwrap();
        let decrypted = decrypt_payload(&encrypted, "folder-password").unwrap();

        assert_eq!(payload, decrypted);
    }

    #[test]
    fn wrong_password_is_rejected() {
        let payload = Payload {
            kind: PayloadKind::File,
            original_name: "wrong-password.txt".to_string(),
            data: b"still secret".to_vec(),
        };

        let encrypted = encrypt_payload(&payload, "correct password").unwrap();
        let error = decrypt_payload(&encrypted, "incorrect password").unwrap_err();

        assert_eq!(
            error.to_string(),
            "Decryption failed. Check the password or confirm the file was created by Encryptallinator."
        );
    }
}
