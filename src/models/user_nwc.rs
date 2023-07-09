use std::str::FromStr;

use bitcoin::hashes::hex::ToHex;
use bitcoin::secp256k1::PublicKey;
use diesel::prelude::*;
use nostr::key::{SecretKey, XOnlyPublicKey};
use nostr::nips::nip47::NostrWalletConnectURI;
use serde::{Deserialize, Serialize};

use super::schema::user_nwc;

#[derive(Queryable, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[diesel(primary_key(request_key))]
#[diesel(table_name = user_nwc)]
pub struct UserNwc {
    request_key: String,
    response_key: String,
    relay_url: String,
    user_pubkey: String,
    date_created: String,
}

impl UserNwc {
    pub(crate) fn user_pubkey(&self) -> PublicKey {
        PublicKey::from_str(&self.user_pubkey).expect("invalid pubkey")
    }

    pub fn nwc_uri(&self) -> NostrWalletConnectURI {
        let public_key = XOnlyPublicKey::from_str(&self.request_key).expect("invalid request key");
        let secret = SecretKey::from_str(&self.response_key).expect("invalid response key");
        let relay_url = self.relay_url.clone().parse().expect("invalid relay url");
        NostrWalletConnectURI {
            public_key,
            secret,
            relay_url,
            lud16: None,
        }
    }

    pub fn create(
        conn: &mut SqliteConnection,
        nwc_uri: NostrWalletConnectURI,
        user_pubkey: PublicKey,
    ) -> Result<Self, diesel::result::Error> {
        let db = Self {
            request_key: nwc_uri.public_key.to_hex(),
            response_key: nwc_uri.secret.secret_bytes().to_hex(),
            relay_url: nwc_uri.relay_url.to_string(),
            user_pubkey: user_pubkey.to_hex(),
            date_created: chrono::Utc::now().naive_utc().to_string(),
        };

        diesel::insert_into(user_nwc::table)
            .values(&db)
            .execute(conn)?;

        Ok(db)
    }

    pub fn find_by_response_key(
        conn: &mut SqliteConnection,
        response_key: &XOnlyPublicKey,
    ) -> Result<Option<Self>, diesel::result::Error> {
        let result = user_nwc::table
            .filter(user_nwc::response_key.eq(response_key.to_hex()))
            .first::<Self>(conn);

        match result {
            Ok(found) => Ok(Some(found)),
            Err(diesel::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn find_by_user(
        conn: &mut SqliteConnection,
        user_pubkey: &PublicKey,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let found = user_nwc::table
            .filter(user_nwc::user_pubkey.eq(user_pubkey.to_hex()))
            .load::<Self>(conn)?;

        Ok(found)
    }
}
