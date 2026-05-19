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
