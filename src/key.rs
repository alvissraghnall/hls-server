#[cfg(feature = "aes")]
mod encrypt {
    use aes::cipher::Iv;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use cbc::Encryptor;

    pub fn encrypt_aes_gcm(plain_text: &[u8], key: &[u8], nonce: &[u8]) -> Vec<u8> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        cipher
            .encrypt(Nonce::from_slice(nonce), plain_text)
            .expect("encryption failure!")
    }

    pub(crate) fn encrypt_aes_cbc(plain_text: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
        use aes::Aes256;
        use cbc::cipher::{BlockModeDecrypt, BlockModeEncrypt, Key, KeyIvInit};


        type Aes256Cbc = cbc::Encryptor<Aes256>;

        let kk = Key::<Encryptor<Aes256>>::try_from(key).unwrap();
        let ivv = Iv::<Encryptor<Aes256>>::try_from(iv).unwrap();

        let mut buffer = [0u8; 48]; // Must account for padding
        let pt_len = plain_text.len();
        buffer[..pt_len].copy_from_slice(plain_text);

        let ct = Aes256Cbc::new(&kk, &ivv)
            .encrypt_padded::<cbc::cipher::block_padding::Pkcs7>(&mut buffer, pt_len)
            .unwrap();
        ct.to_vec()
    }
}

#[cfg(feature = "aes")]
mod decrypt {
    use aes::cipher::Iv;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};

    pub fn decrypt_aes_gcm(cipher_text: &[u8], key: &[u8], nonce: &[u8]) -> Vec<u8> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        cipher
            .decrypt(Nonce::from_slice(nonce), cipher_text)
            .expect("decryption failure!")
    }

    pub fn decrypt_aes_cbc(cipher_text: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
        use aes::Aes256;
        use cbc::cipher::{BlockModeDecrypt, Key, KeyIvInit};

        type Aes256Cbc = cbc::Decryptor<Aes256>;

        let kk = Key::<Aes256Cbc>::try_from(key).unwrap();
        let ivv = Iv::<Aes256Cbc>::try_from(iv).unwrap();

        let mut buffer = [0u8; 48]; // Must account for padding
        let ct_len = cipher_text.len();
        buffer[..ct_len].copy_from_slice(cipher_text);

        let pt = Aes256Cbc::new(&kk, &ivv)
            .decrypt_padded::<cbc::cipher::block_padding::Pkcs7>(&mut buffer)
            .unwrap();
        pt.to_vec()
    }
}
