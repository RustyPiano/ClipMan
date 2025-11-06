use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

const NONCE_LEN: usize = 12;

pub struct Crypto {
    key: LessSafeKey,
    rng: SystemRandom,
}

impl Crypto {
    pub fn new(key_bytes: &[u8; 32]) -> Self {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key_bytes).unwrap();
        let key = LessSafeKey::new(unbound_key);
        let rng = SystemRandom::new();

        Self { key, rng }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng
            .fill(&mut nonce_bytes)
            .map_err(|e| format!("Failed to generate nonce: {:?}", e))?;

        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut in_out = data.to_vec();
        self.key
            .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;

        // Prepend nonce to encrypted data
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&in_out);

        Ok(result)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
        if encrypted_data.len() < NONCE_LEN {
            return Err("Invalid encrypted data".to_string());
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_LEN);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes.try_into().map_err(|_| "Invalid nonce")?,
        );

        let mut in_out = ciphertext.to_vec();
        let plaintext = self
            .key
            .open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|e| format!("Decryption failed: {:?}", e))?;

        Ok(plaintext.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let crypto = Crypto::new(&key);

        let data = b"Hello, ClipMan!";
        let encrypted = crypto.encrypt(data).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}
