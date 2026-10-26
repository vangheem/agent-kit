use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub use crate::error::CryptoError;

const NONCE_LEN: usize = 12;

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    to_hex(&Sha256::digest(token.as_bytes()))
}

pub fn token_prefix(token: &str) -> String {
    token.chars().take(8).collect()
}

pub fn seal(secret: &str, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = cipher(secret)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn open(secret: &str, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < NONCE_LEN {
        return Err(CryptoError::Decrypt);
    }
    let (nonce_bytes, encrypted) = ciphertext.split_at(NONCE_LEN);
    let cipher = cipher(secret)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, encrypted)
        .map_err(|_| CryptoError::Decrypt)
}

fn cipher(secret: &str) -> Result<Aes256Gcm, CryptoError> {
    let key = Sha256::digest(secret.as_bytes());
    Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::Encrypt)
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}
