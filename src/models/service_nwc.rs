use std::str::FromStr;

use bitcoin::hashes::hex::ToHex;
use bitcoin::secp256k1::PublicKey;
use diesel::prelude::*;
use diesel::result::Error::DeserializationError;
use nostr::key::{SecretKey, XOnlyPublicKey};
use nostr::nips::nip47::NostrWalletConnectURI;
use nostr::Keys;
use serde::{Deserialize, Serialize};

use super::schema::service_nwc;

#[derive(Queryable, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[diesel(primary_key(request_key))]
#[diesel(table_name = service_nwc)]
pub struct ServiceNwc {
    request_key: String,
    response_key: String,
    relay_url: String,
    service_name: String,
    user_pubkey: String,
    date_created: String,
}

impl ServiceNwc {
    pub fn generate(user_pubkey: PublicKey, service_name: String) -> ServiceNwc {
        let request_key = Keys::generate();
        let response_key = Keys::generate();

        ServiceNwc {
            request_key: request_key.public_key().to_hex(),
            response_key: response_key.secret_key().unwrap().secret_bytes().to_hex(),
            relay_url: "wss://relay.damus.io".to_string(),
            service_name,
            user_pubkey: user_pubkey.to_hex(),
            date_created: chrono::Utc::now().naive_utc().to_string(),
        }
    }

    pub fn user_pubkey(&self) -> PublicKey {
        PublicKey::from_str(&self.user_pubkey).expect("invalid pubkey")
    }

    pub fn request_key(&self) -> XOnlyPublicKey {
        XOnlyPublicKey::from_str(&self.request_key).expect("invalid request key")
    }

    pub fn response_key(&self) -> SecretKey {
        SecretKey::from_str(&self.response_key).expect("invalid response key")
    }

    pub fn nwc_uri(&self) -> NostrWalletConnectURI {
        let relay_url = self.relay_url.clone().parse().expect("invalid relay url");
        NostrWalletConnectURI {
            public_key: self.request_key(),
            secret: self.response_key(),
            relay_url,
            lud16: None,
        }
    }

    pub fn insert(conn: &mut SqliteConnection, db: &Self) -> Result<(), diesel::result::Error> {
        diesel::insert_into(service_nwc::table)
            .values(db)
            .execute(conn)?;

        Ok(())
    }

    pub fn find_by_request_key(
        conn: &mut SqliteConnection,
        request_key: &XOnlyPublicKey,
    ) -> Result<Option<Self>, diesel::result::Error> {
        let result = service_nwc::table
            .filter(service_nwc::request_key.eq(request_key.to_hex()))
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
        let found = service_nwc::table
            .filter(service_nwc::user_pubkey.eq(user_pubkey.to_hex()))
            .load::<Self>(conn)?;

        Ok(found)
    }

    pub fn get_all_keys(
        conn: &mut SqliteConnection,
    ) -> Result<Vec<XOnlyPublicKey>, diesel::result::Error> {
        let found = service_nwc::table
            .select(service_nwc::request_key)
            .load::<String>(conn)?;

        let mut keys = vec![];
        for str in found {
            let key =
                XOnlyPublicKey::from_str(&str).map_err(|e| DeserializationError(Box::new(e)))?;
            keys.push(key);
        }

        Ok(keys)
    }
}
