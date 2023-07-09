use std::str::FromStr;

use bitcoin::hashes::hex::ToHex;
use bitcoin::secp256k1::PublicKey;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::schema::users;

#[derive(Queryable, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[diesel(primary_key(pubkey))]
pub struct User {
    pubkey: String,
    date_created: String,
}

impl User {
    pub(crate) fn pubkey(&self) -> PublicKey {
        PublicKey::from_str(&self.pubkey).expect("invalid pubkey")
    }

    pub fn create(
        conn: &mut SqliteConnection,
        pubkey: PublicKey,
    ) -> Result<Self, diesel::result::Error> {
        let user = Self {
            pubkey: pubkey.to_hex(),
            date_created: chrono::Utc::now().naive_utc().to_string(),
        };

        diesel::insert_into(users::table)
            .values(&user)
            .execute(conn)?;

        Ok(user)
    }

    pub fn find(
        conn: &mut SqliteConnection,
        pubkey: &PublicKey,
    ) -> Result<Option<Self>, diesel::result::Error> {
        let result = users::table
            .filter(users::pubkey.eq(pubkey.to_hex()))
            .first::<Self>(conn);

        match result {
            Ok(found) => Ok(Some(found)),
            Err(diesel::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
