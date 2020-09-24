use base64::URL_SAFE;
use sodiumoxide::crypto::aead::xchacha20poly1305_ietf::gen_key;

fn main() {
    let secret_key = gen_key();
    let encoded = base64::encode_config(secret_key.as_ref(), URL_SAFE);
    println!("{}", encoded);
}
