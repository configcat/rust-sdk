use sha1::{Digest, Sha1};

pub fn sha1(payload: String) -> String {
    let hash = Sha1::digest(payload.as_bytes());
    base16ct::lower::encode_string(&hash)
}
