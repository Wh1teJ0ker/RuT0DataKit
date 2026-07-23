//! 本地加密 / 编码工具集（T13-1）。
//!
//! 提供三种可逆变换的纯本地实现（无任何网络依赖）：
//! - `EncryptAlgo::AesCbc`：AES-CBC（PKCS7 填充），密钥由调用方传入的字符串经
//!   SHA-256 派生后取前 32 字节（AES-256）。IV 随机生成（16 字节，`rand::rngs::OsRng`），
//!   密文格式为 `base64(IV[16] || ciphertext)`；解密时从 Base64 前缀解析 IV。
//!   调用方传入的 `iv` 参数为空时随机生成，否则按原样使用（仍需 16 字节）。
//! - `EncryptAlgo::Base64`：标准 Base64 编解码，非法输入返回 `Err`。
//! - `EncryptAlgo::Hex`：大小写不敏感解码，编码输出小写；非法输入返回 `Err`。
//!
//! 统一 API：`encrypt_text(algo, plaintext, key, iv)` / `decrypt_text(algo, ciphertext, key, iv)`。
//! 对 Base64 / Hex，`key` 与 `iv` 参数忽略。

use aes::Aes256;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// 加密 / 编码算法选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptAlgo {
    /// AES-256-CBC（PKCS7），密文格式 `base64(IV || ciphertext)`。
    AesCbc,
    /// 标准 Base64。
    Base64,
    /// 十六进制（输出小写）。
    Hex,
}

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// AES 固定使用 32 字节密钥（AES-256）。
const AES_KEY_LEN: usize = 32;
/// AES-CBC 固定 16 字节 IV。
const AES_IV_LEN: usize = 16;

/// 由用户 key 字符串派生 AES-256 密钥（SHA-256 取前 32 字节）。
fn derive_key(key: &str) -> [u8; AES_KEY_LEN] {
    let digest = Sha256::digest(key.as_bytes());
    let mut out = [0u8; AES_KEY_LEN];
    out.copy_from_slice(&digest[..AES_KEY_LEN]);
    out
}

/// 加密 / 编码文本。
///
/// - `algo`：算法。
/// - `plaintext`：原始字符串。
/// - `key`：AES 必填（任意长度字符串，内部 SHA-256 派生）；Base64/Hex 忽略。
/// - `iv`：AES 的 IV；空串表示随机生成；非空必须是 16 字节。Base64/Hex 忽略。
///
/// 返回：
/// - AES-CBC：`base64(IV || ciphertext)`
/// - Base64：Base64 字符串
/// - Hex：小写 hex 字符串
pub fn encrypt_text(
    algo: EncryptAlgo,
    plaintext: &str,
    key: &str,
    iv: &str,
) -> Result<String, String> {
    match algo {
        EncryptAlgo::AesCbc => {
            if key.is_empty() {
                return Err("AES key must not be empty".to_string());
            }
            let mut iv_bytes = [0u8; AES_IV_LEN];
            if iv.is_empty() {
                rand::rngs::OsRng.fill_bytes(&mut iv_bytes);
            } else {
                let iv_raw = iv.as_bytes();
                if iv_raw.len() != AES_IV_LEN {
                    return Err(format!(
                        "AES IV must be {} bytes, got {}",
                        AES_IV_LEN,
                        iv_raw.len()
                    ));
                }
                iv_bytes.copy_from_slice(iv_raw);
            }
            let key_bytes = derive_key(key);
            let pt = plaintext.as_bytes();
            let mut buf = vec![0u8; pt.len() + 16]; // PKCS7 至多扩 1 块
            let ct_len = Aes256CbcEnc::new(&key_bytes.into(), &iv_bytes.into())
                .encrypt_padded_b2b_mut::<Pkcs7>(pt, &mut buf)
                .map_err(|e| format!("aes encrypt failed: {e}"))?;
            let mut combined = Vec::with_capacity(AES_IV_LEN + ct_len.len());
            combined.extend_from_slice(&iv_bytes);
            combined.extend_from_slice(ct_len);
            Ok(B64.encode(&combined))
        }
        EncryptAlgo::Base64 => Ok(B64.encode(plaintext.as_bytes())),
        EncryptAlgo::Hex => Ok(hex::encode(plaintext.as_bytes())),
    }
}

/// 解密 / 解码文本。
///
/// - `algo`：算法。
/// - `ciphertext`：密文 / 编码串。
/// - `key`：AES 必填（与加密时一致的字符串）；Base64/Hex 忽略。
/// - `iv`：AES 忽略（IV 已内嵌于密文前缀）；保留参数为 API 对称。
pub fn decrypt_text(
    algo: EncryptAlgo,
    ciphertext: &str,
    key: &str,
    _iv: &str,
) -> Result<String, String> {
    match algo {
        EncryptAlgo::AesCbc => {
            if key.is_empty() {
                return Err("AES key must not be empty".to_string());
            }
            let combined = B64
                .decode(ciphertext.trim())
                .map_err(|e| format!("base64 decode failed: {e}"))?;
            if combined.len() < AES_IV_LEN + 1 {
                return Err(format!(
                    "ciphertext too short: need >= {} bytes, got {}",
                    AES_IV_LEN + 1,
                    combined.len()
                ));
            }
            let (iv_bytes, ct) = combined.split_at(AES_IV_LEN);
            let mut iv_arr = [0u8; AES_IV_LEN];
            iv_arr.copy_from_slice(iv_bytes);
            let key_bytes = derive_key(key);
            let mut buf = ct.to_vec();
            let pt = Aes256CbcDec::new(&key_bytes.into(), &iv_arr.into())
                .decrypt_padded_mut::<Pkcs7>(&mut buf)
                .map_err(|e| format!("aes decrypt failed: {e}"))?;
            String::from_utf8(pt.to_vec())
                .map_err(|e| format!("utf8 decode failed: {e}"))
        }
        EncryptAlgo::Base64 => {
            let bytes = B64
                .decode(ciphertext.trim())
                .map_err(|e| format!("base64 decode failed: {e}"))?;
            String::from_utf8(bytes).map_err(|e| format!("utf8 decode failed: {e}"))
        }
        EncryptAlgo::Hex => {
            let bytes = hex::decode(ciphertext).map_err(|e| format!("hex decode failed: {e}"))?;
            String::from_utf8(bytes).map_err(|e| format!("utf8 decode failed: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(algo: EncryptAlgo, plaintext: &str, key: &str, iv: &str) {
        let ct = encrypt_text(algo, plaintext, key, iv).expect("encrypt");
        let pt = decrypt_text(algo, &ct, key, iv).expect("decrypt");
        assert_eq!(pt, plaintext, "roundtrip failed for algo={algo:?}");
    }

    #[test]
    fn aes_cbc_roundtrip_random_iv() {
        roundtrip(EncryptAlgo::AesCbc, "hello world", "my-secret-key", "");
        roundtrip(EncryptAlgo::AesCbc, "", "my-secret-key", "");
        roundtrip(
            EncryptAlgo::AesCbc,
            "中文与特殊字符 !@#$%^&*()_+-=",
            "key-1",
            "",
        );
    }

    #[test]
    fn aes_cbc_roundtrip_fixed_iv() {
        // 固定 16 字节 IV
        let iv = "0123456789abcdef";
        assert_eq!(iv.as_bytes().len(), 16);
        roundtrip(EncryptAlgo::AesCbc, "payload", "k", iv);
    }

    #[test]
    fn aes_cbc_wrong_key_fails_or_differs() {
        let ct = encrypt_text(EncryptAlgo::AesCbc, "secret", "right-key", "").unwrap();
        // 错误 key 应解密失败（PKCS7 padding 校验或 UTF8 失败）
        assert!(decrypt_text(EncryptAlgo::AesCbc, &ct, "wrong-key", "").is_err());
    }

    #[test]
    fn aes_cbc_empty_key_errors() {
        assert!(encrypt_text(EncryptAlgo::AesCbc, "x", "", "").is_err());
        let ct = encrypt_text(EncryptAlgo::AesCbc, "x", "k", "").unwrap();
        assert!(decrypt_text(EncryptAlgo::AesCbc, &ct, "", "").is_err());
    }

    #[test]
    fn aes_cbc_bad_iv_length_errors() {
        assert!(encrypt_text(EncryptAlgo::AesCbc, "x", "k", "short").is_err());
    }

    #[test]
    fn aes_cbc_bad_ciphertext_errors() {
        assert!(decrypt_text(EncryptAlgo::AesCbc, "not-base64!!!", "k", "").is_err());
        // 合法 base64 但长度不足
        assert!(decrypt_text(EncryptAlgo::AesCbc, "AAAA", "k", "").is_err());
    }

    #[test]
    fn aes_cbc_iv_embedded_in_output() {
        // 同一明文 + 同一 key，随机 IV 时两次密文应不同
        let a = encrypt_text(EncryptAlgo::AesCbc, "same", "k", "").unwrap();
        let b = encrypt_text(EncryptAlgo::AesCbc, "same", "k", "").unwrap();
        assert_ne!(a, b, "random IV should yield different ciphertexts");
        // 但都能还原
        assert_eq!(decrypt_text(EncryptAlgo::AesCbc, &a, "k", "").unwrap(), "same");
        assert_eq!(decrypt_text(EncryptAlgo::AesCbc, &b, "k", "").unwrap(), "same");
    }

    #[test]
    fn base64_roundtrip() {
        roundtrip(EncryptAlgo::Base64, "hello", "ignored", "");
        roundtrip(EncryptAlgo::Base64, "", "ignored", "");
        roundtrip(EncryptAlgo::Base64, "中文 !@#", "ignored", "");
    }

    #[test]
    fn base64_known_value() {
        // "hello" -> base64 "aGVsbG8="
        assert_eq!(encrypt_text(EncryptAlgo::Base64, "hello", "", "").unwrap(), "aGVsbG8=");
    }

    #[test]
    fn base64_invalid_input_errors() {
        assert!(decrypt_text(EncryptAlgo::Base64, "!!!not base64!!!", "", "").is_err());
        // 非法 UTF8（Base64 解码后是 0xFF，不是合法 UTF8 起始）
        assert!(decrypt_text(EncryptAlgo::Base64, "/w==", "", "").is_err());
    }

    #[test]
    fn hex_roundtrip() {
        roundtrip(EncryptAlgo::Hex, "hello", "ignored", "");
        roundtrip(EncryptAlgo::Hex, "", "ignored", "");
        roundtrip(EncryptAlgo::Hex, "中文 !@#", "ignored", "");
    }

    #[test]
    fn hex_lowercase_output() {
        let ct = encrypt_text(EncryptAlgo::Hex, "ab", "", "").unwrap();
        assert_eq!(ct, ct.to_lowercase());
        assert!(!ct.chars().any(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn hex_case_insensitive_decode() {
        // 大写 hex 也应能解码
        let upper = encrypt_text(EncryptAlgo::Hex, "Ab", "", "").unwrap().to_uppercase();
        assert_eq!(decrypt_text(EncryptAlgo::Hex, &upper, "", "").unwrap(), "Ab");
    }

    #[test]
    fn hex_invalid_input_errors() {
        assert!(decrypt_text(EncryptAlgo::Hex, "xyz", "", "").is_err());
        assert!(decrypt_text(EncryptAlgo::Hex, "abc", "", "").is_err()); // 奇数长度
    }
}
