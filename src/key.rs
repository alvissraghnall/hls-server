#[cfg(feature = "aes")]
pub mod encrypt {
    use aes::cipher::{
        BlockModeEncrypt, Iv, Key as AesKey, KeyIvInit, block_padding::Pkcs7, consts::U16,
    };
    use aes_gcm::{AesGcm, Key as AesGcmKey, KeyInit, Nonce, aead::Aead, aes::Aes256};
    use cbc::Encryptor;

    use crate::error::KeyError;

    type Aes256GcmCustomNonce = AesGcm<Aes256, U16>;

    pub fn encrypt_aes_128(plain_text: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, KeyError> {
        use aes::Aes128;

        type Aes128CbcEnc = cbc::Encryptor<Aes128>;

        if key.len() != 16 {
            return Err(KeyError {
                message: "AES-128 key must be 16 bytes".to_string(),
            });
        }
        if iv.len() != 16 {
            return Err(KeyError {
                message: "AES-128 IV must be 16 bytes".to_string(),
            });
        }

        let pt_len = plain_text.len();
        let mut buffer = vec![0u8; pt_len + 16]; // Must account for padding
        let ct = {
            let key = AesKey::<Encryptor<Aes128>>::try_from(key).map_err(|_| KeyError {
                message: "Invalid AES-128 key".to_string(),
            })?;
            let iv = Iv::<Encryptor<Aes128>>::try_from(iv).map_err(|_| KeyError {
                message: "Invalid AES-128 IV".to_string(),
            })?;

            Aes128CbcEnc::new(&key, &iv)
        };
        Ok(Vec::from(
            ct.encrypt_padded_b2b::<Pkcs7>(plain_text, &mut buffer)?,
        ))
    }

    pub fn encrypt_aes_256_gcm(
        plain_text: &[u8],
        key: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>, KeyError> {
        let cipher = Aes256GcmCustomNonce::new(AesGcmKey::<Aes256GcmCustomNonce>::from_slice(key));

        let ciphertext = cipher.encrypt(Nonce::from_slice(nonce), plain_text)?;
        let mut out = Vec::with_capacity(nonce.len() + ciphertext.len());
        out.extend_from_slice(nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }
}

#[cfg(feature = "aes")]
pub mod decrypt {
    use aes::{
        Aes128,
        cipher::{BlockModeDecrypt, Iv, Key as AesKey, KeyIvInit, consts::U16},
    };
    use aes_gcm::{Aes256Gcm, AesGcm, Key, KeyInit, Nonce, aead::Aead, aes::Aes256};
    use cbc::Decryptor;

    use crate::error::KeyError;
    type Aes256GcmCustomNonce = AesGcm<Aes256, U16>;
    type Aes128CbcDec = cbc::Decryptor<Aes128>;

    pub fn decrypt_aes_128(
        cipher_text: &[u8],
        key: &[u8],
        iv: &[u8],
    ) -> Result<Vec<u8>, KeyError> {
        let key = AesKey::<Decryptor<Aes128>>::try_from(key).unwrap();
        let iv = Iv::<Decryptor<Aes128>>::try_from(iv).unwrap();

        let mut buffer = vec![0u8; cipher_text.len() + 16];

        let pt = Aes128CbcDec::new(&key, &iv)
            .decrypt_padded_b2b::<cbc::cipher::block_padding::Pkcs7>(cipher_text, &mut buffer)
            .map_err(|e| KeyError {
                message: e.to_string(),
            })?;
        Ok(Vec::from(pt))
    }

    pub fn decrypt_aes_256_gcm(cipher_text: &[u8], key: &[u8]) -> Result<Vec<u8>, KeyError> {
        let cipher = Aes256GcmCustomNonce::new(Key::<Aes256Gcm>::from_slice(key));

        // 16 byte IV + 16 byte tag AT VERY LEAST
        if cipher_text.len() < 32 {
            return Err(KeyError {
                message: "Cipher text too short".to_string(),
            });
        }

        // read first 16 bytes as the IV
        let iv = &cipher_text[..16];
        let cipher_text = &cipher_text[16..];

        let pt = cipher
            .decrypt(Nonce::from_slice(iv), cipher_text)
            .map_err(|e| KeyError {
                message: e.to_string(),
            })?;
        Ok(pt)
    }
}

#[cfg(test)]
mod tests {

    use hex_literal::hex;

    use super::{
        decrypt::{decrypt_aes_128, decrypt_aes_256_gcm},
        encrypt::{encrypt_aes_128, encrypt_aes_256_gcm},
    };

    #[test]
    fn test_encrypt_decrypt_aes_128() {
        let plain_text = b"Hello, World!";
        let key = [0x42; 16];
        let iv = [0x24; 16];

        let cipher_text = encrypt_aes_128(plain_text, &key, &iv).unwrap();
        let decrypted_text = decrypt_aes_128(&cipher_text, &key, &iv).unwrap();

        assert_eq!(plain_text, decrypted_text.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_aes_128_with_cipher_check() {
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let plaintext = *b"hello world! this is my plaintext.";

        let ct = hex!(
            "c7fe247ef97b21f07cbdd26cb5d346bf"
            "d27867cb00d9486723e159978fb9a5f9"
            "14cfb228a710de4171e396e7b6cf859e"
        );

        let cipher_text = encrypt_aes_128(&plaintext, &key, &iv).unwrap();

        assert_eq!(cipher_text, &ct[..]);
        let decrypted_text = decrypt_aes_128(&cipher_text, &key, &iv).unwrap();

        assert_eq!(plaintext, decrypted_text.as_slice());
    }

    #[test]
    fn test_aes_256_gcm() {
        let plain_text = b"Hello, World!";
        let key = [0x42; 32];
        let nonce = [0x24; 16];

        let cipher_text = encrypt_aes_256_gcm(plain_text, &key, &nonce).unwrap();
        let decrypted_text = decrypt_aes_256_gcm(&cipher_text, &key).unwrap();

        assert_eq!(plain_text, decrypted_text.as_slice());
    }
}
