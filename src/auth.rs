use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Returns true if `digest` equals the hex-encoded HMAC-SHA256 of `body` keyed with `api_key`.
///
/// This matches the digest authentication used by the original Python matrix-webhook:
///   hmac.new(api_key.encode(), request_body, sha256).hexdigest()
pub fn verify_hmac(body: &[u8], api_key: &str, digest: &str) -> bool {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(api_key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());
    // Constant-time comparison via hex strings (both are fixed-length hex)
    expected == digest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_digest(body: &[u8], key: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap();
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn correct_digest_accepted() {
        let body = b"{\"body\":\"hello\"}";
        let digest = make_digest(body, "secret");
        assert!(verify_hmac(body, "secret", &digest));
    }

    #[test]
    fn wrong_digest_rejected() {
        assert!(!verify_hmac(b"hello", "secret", "deadbeef"));
    }

    #[test]
    fn wrong_key_rejected() {
        let body = b"hello";
        let digest = make_digest(body, "correct-key");
        assert!(!verify_hmac(body, "wrong-key", &digest));
    }

    #[test]
    fn tampered_body_rejected() {
        let digest = make_digest(b"original body", "key");
        assert!(!verify_hmac(b"tampered body", "key", &digest));
    }

    #[test]
    fn empty_body_accepted() {
        let digest = make_digest(b"", "key");
        assert!(verify_hmac(b"", "key", &digest));
    }
}
