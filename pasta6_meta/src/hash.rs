use std::error::Error;

use bytes::BytesMut;
use sodiumoxide::crypto::pwhash::argon2id13::{
    pwhash, pwhash_verify, HashedPassword, MEMLIMIT_MODERATE, OPSLIMIT_MODERATE,
};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

// Newtype `String` to ensure that we always pad/unpad password hashes correctly.
#[derive(Debug)]
pub(crate) struct Hash(String);

impl ToSql for &Hash {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        (self.0).to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <String as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for Hash {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Hash(<String as FromSql>::from_sql(ty, raw)?))
    }

    fn accepts(ty: &Type) -> bool {
        <String as FromSql>::accepts(ty)
    }
}

pub(crate) fn hash(password: &str) -> Hash {
    let hash = pwhash(password.as_bytes(), OPSLIMIT_MODERATE, MEMLIMIT_MODERATE).unwrap();
    // From: https://libsodium.gitbook.io/doc/password_hashing/default_phf#password-storage
    // > The output string is zero-terminated, includes only ASCII characters and can be safely
    // > stored into SQL databases and other data stores. No extra information has to be
    // > stored in order to verify the password.
    // Since the output string is ASCII encoded, we can safely convert to a UTF-8 string
    // and unwrap. We trim the trailing NULL characters so we don't accidentally store
    // them in the DB.
    // TODO: what happens if we _do_ panic? Does a 5xx get shown to the user?
    assert!(hash.0.len() <= 128);
    Hash(
        std::str::from_utf8(&hash.0)
            .unwrap()
            .trim_end_matches('\u{0}')
            .to_string(),
    )
}

pub(crate) fn verify(hash: &Hash, passwd: &str) -> bool {
    assert!(hash.0.len() <= 128);
    let padded_hash = pad_password_hash(&hash.0);
    match HashedPassword::from_slice(&padded_hash) {
        Some(hp) => pwhash_verify(&hp, passwd.as_bytes()),
        _ => false,
    }
}

// The slice or array provided to `HashedPassword::from_slice` MUST have a size of 128.
// If it does not, the verification will fail even with a correct password.
// To satisfy the requirement, we pad our hash string with the NULL character.
fn pad_password_hash(hash: &str) -> [u8; 128] {
    let mut padded = [0u8; 128];
    padded[..hash.len()].copy_from_slice(hash.as_bytes());
    padded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        const PASSWORD: &str = "hunter2";
        let hash = hash(PASSWORD);
        assert!(verify(&hash, PASSWORD));
    }
}
