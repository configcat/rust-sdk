use semver::{Error, Version};
use sha1::{Digest, Sha1};
use sha2::Sha256;

pub fn sha1(payload: &str) -> String {
    let hash = Sha1::digest(payload);
    base16ct::lower::encode_string(&hash)
}

pub fn sha256(payload: &str, salt: &str, ctx_salt: &str) -> String {
    let mut cont = String::with_capacity(payload.len() + salt.len() + ctx_salt.len());
    cont.push_str(payload);
    cont.push_str(salt);
    cont.push_str(ctx_salt);
    let hash = Sha256::digest(cont);
    base16ct::lower::encode_string(&hash)
}

pub fn parse_semver(input: &str) -> Result<Version, Error> {
    let mut input_mut = input.trim();
    if let Some((first, _)) = input.split_once('+') {
        input_mut = first;
    }
    Version::parse(input_mut)
}

#[cfg(test)]
mod utils_test {
    use crate::utils::parse_semver;
    use crate::utils::sha1;
    use crate::utils::sha256;

    #[test]
    fn hash() {
        assert_eq!(
            sha1("test_payload"),
            "683231cec21572ae3afd898a1b1487f6b9193ebb"
        );
        assert_eq!(
            sha256("test_payload", "salt", "ctx_salt"),
            "5ee9b44b3b90bedd9441b256c429f862ceb2ea847a58a8e33d8052da141e47aa"
        );
    }

    #[test]
    fn semver_ignore_build_meta() {
        assert!(parse_semver("1.0.0-alpha+build.1")
            .unwrap()
            .eq(&parse_semver("1.0.0-alpha").unwrap()));
    }
}
