use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const STEP_SECS: i64 = 30;
const DIGITS: u32 = 6;

pub fn generate_secret() -> String {
    let mut bytes = [0u8; 20];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

pub fn verify_code(secret: &str, code: &str, now: i64) -> bool {
    let code = code.trim().replace(' ', "");
    if code.len() != DIGITS as usize || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Some(key) = base32_decode(secret).filter(|key| !key.is_empty()) else {
        return false;
    };
    let step = now.div_euclid(STEP_SECS);
    for offset in -1..=1 {
        let counter = (step + offset).max(0) as u64;
        let expected = format!("{:0width$}", hotp(&key, counter), width = DIGITS as usize);
        if constant_time_eq(expected.as_bytes(), code.as_bytes()) {
            return true;
        }
    }
    false
}

pub fn otpauth_url(issuer: &str, account: &str, secret: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        percent_encode(issuer),
        percent_encode(account),
        secret,
        percent_encode(issuer),
        DIGITS,
        STEP_SECS
    )
}

fn hotp(key: &[u8], counter: u64) -> u32 {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | digest[offset + 3] as u32;
    binary % 10u32.pow(DIGITS)
}

fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for ch in input.chars() {
        if matches!(ch, '=' | ' ' | '-' | '_') {
            continue;
        }
        let value = match ch.to_ascii_uppercase() {
            letter @ 'A'..='Z' => letter as u32 - 'A' as u32,
            digit @ '2'..='7' => digit as u32 - '2' as u32 + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_rfc6238_vectors() {
        // RFC 6238 test secret "12345678901234567890" in base32.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert!(verify_code(secret, "287082", 59));
        assert!(verify_code(secret, "081804", 1111111109));
        assert!(!verify_code(secret, "287082", 3600 * 48));
    }

    #[test]
    fn secret_round_trip() {
        let secret = generate_secret();
        assert_eq!(secret.len(), 32);
        let decoded = base32_decode(&secret).unwrap();
        assert_eq!(decoded.len(), 20);
    }
}
