use crate::paste::models::Paste;
use deadpool_postgres::Client;
use base64::{URL_SAFE_NO_PAD, decode_config, encode_config};
use rand::{thread_rng, Rng};
use std::{convert::TryFrom, convert::TryInto, error::Error, fmt::Debug, fmt::Display, str::FromStr};

macro_rules! paste_table {
    () => {
        "paste"
    };
}

pub(crate) async fn init_db(client: &Client) -> Result<(), tokio_postgres::Error> {
    const INIT_SQL: [&str; 2] = [
        r#"
        CREATE TABLE IF NOT EXISTS paste
        (
            id SERIAL PRIMARY KEY NOT NULL,
            created_at timestamp with time zone NOT NULL DEFAULT (now() at time zone 'utc'),
            hash bytea UNIQUE NOT NULL CHECK(length(hash) = 20),
            data bytea NOT NULL
        )"#,
        r#"
        CREATE TABLE IF NOT EXISTS "user"
        (
            id SERIAL PRIMARY KEY NOT NULL,
            created_at timestamp with time zone NOT NULL DEFAULT (now() at time zone 'utc'),
            username TEXT UNIQUE NOT NULL CHECK(length(username) <= 15)
        )
        "#,
    ];

    for query in &INIT_SQL {
        let _rows = client.query(*query, &[]).await?;
    }

    Ok(())
}

// TODO: all of these leave data in the DB if anything fails immediately after the INSERT
//   (e.g. parsing the query response data). Should we add a COMMIT call immediately before
//   returning? This would add a round trip to PostgreSQL but avoid leaving data in the DB
//   unless the query was handled successfully. Though perhaps this is better done at the
//   handler layer.
pub(crate) async fn create_paste(
    client: &Client,
    body: &[u8],
) -> Result<Paste, tokio_postgres::Error> {
    const QUERY: &str = concat!(
        "INSERT INTO ",
        paste_table!(),
        " (data, hash) VALUES ($1, $2) RETURNING *"
    );
    let hash = Hash::new();
    let row = client.query_one(QUERY, &[&body, &hash.decoded()]).await?;
    Ok(row_to_paste(&row))
}

pub(crate) async fn get_paste(client: &Client, hash: Hash) -> Result<Paste, tokio_postgres::Error> {
    const QUERY: &str = concat!(
        "SELECT id, created_at, hash, data FROM ",
        paste_table!(),
        " WHERE hash = $1"
    );
    let row = client.query_one(QUERY, &[&hash.decoded()]).await?;
    Ok(row_to_paste(&row))
}

fn row_to_paste(row: &tokio_postgres::row::Row) -> Paste {
    let id = row.get(0);
    let created_at = row.get(1);
    let hash: Hash = row.get::<_, &[u8]>(2).try_into().expect("could not parse vec to hash");
    let data = row.get(3);
    Paste::new(id, created_at, hash, data)
}

/// A URL safe base64 encoded representation of a 160-bit random identifier.
///
/// Uses 20 byte (160-bit) random identifiers to provide 80 bits of security
/// against collision attacks. This prevents attacks from enumerating paste identifiers
/// by brute-forcing requests against the API. Short identifiers combined with rate
/// limiting would stop most attackers, but wouldn't stop a botnet. This should stop a
/// botnet. Ideally this would still be combined with rate limiting for extra security.
///
/// Base64 requires `4*(n/3)` chars to represent `n` bytes. For our 20 byte identifiers
/// we need `4*(20/3)~=26.666` chars. Since we don't use padding we don't need to round
/// this up to a multiple of 4, but we still need to round it up to 27 chars.
#[derive(Debug)]
pub(crate) struct Hash(String);

impl Hash {
    fn new() -> Self {
        let bytes = thread_rng().gen::<[u8; 20]>();
        let encoded = encode_config(bytes, URL_SAFE_NO_PAD);
        Self(encoded)
    }

    fn decoded(&self) -> Vec<u8> {
        decode_config(&self.0, URL_SAFE_NO_PAD).unwrap()
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> TryFrom<&'a [u8]> for Hash {
    type Error = String;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() != 20 {
            return Err(format!("expected Vec of len 20 got {}", value.len()));
        }
        let encoded = encode_config(value, URL_SAFE_NO_PAD);
        Ok(Self(encoded))
    }
}

impl FromStr for Hash {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 27 {
            return Err(format!("expected str of len 27 got {}", s.len()))?;
        }
        decode_config(s, URL_SAFE_NO_PAD)?;
        Ok(Self(s.to_owned()))
    }
}