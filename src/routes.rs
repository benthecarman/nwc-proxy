use crate::models::service_nwc::ServiceNwc;
use crate::models::user::User;
use crate::models::user_nwc::UserNwc;
use crate::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use bitcoin::secp256k1::PublicKey;
use nostr::nips::nip47::NostrWalletConnectURI;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub(crate) fn handle_anyhow_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, format!("{err}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetUserNwcRequest {
    pub user_pubkey: PublicKey, // todo use actual auth
    nwc: String,
}

impl SetUserNwcRequest {
    pub fn nwc(&self) -> Option<NostrWalletConnectURI> {
        NostrWalletConnectURI::from_str(&self.nwc).ok()
    }
}

pub(crate) fn set_user_nwc_impl(payload: SetUserNwcRequest, state: &State) -> anyhow::Result<()> {
    match payload.nwc() {
        Some(nwc) => {
            let conn = &mut state.db_pool.get()?;
            let _ = User::create(conn, payload.user_pubkey)?;
            let _ = UserNwc::create(conn, nwc.clone(), payload.user_pubkey)?;

            println!("New user: {}!", payload.user_pubkey);
            // notify new key
            let keys = state.pubkeys.lock().unwrap();
            keys.send_if_modified(|current| {
                if current.contains(&nwc.public_key) {
                    false
                } else {
                    current.push(nwc.public_key);
                    true
                }
            });

            Ok(())
        }
        None => Err(anyhow::anyhow!("Invalid NWC")),
    }
}

pub async fn set_user_nwc(
    Extension(state): Extension<State>,
    Json(payload): Json<SetUserNwcRequest>,
) -> Result<Json<()>, (StatusCode, String)> {
    match set_user_nwc_impl(payload, &state) {
        Ok(_) => Ok(Json(())),
        Err(e) => Err(handle_anyhow_error(e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetServiceNwcRequest {
    pub user_pubkey: PublicKey, // todo use actual auth
    service_name: String,
}

pub(crate) fn get_service_nwc_impl(
    user_pubkey: PublicKey,
    service_name: String,
    state: &State,
) -> anyhow::Result<NostrWalletConnectURI> {
    let service_nwc = ServiceNwc::generate(user_pubkey, service_name);
    let conn = &mut state.db_pool.get()?;
    ServiceNwc::insert(conn, &service_nwc)?;

    println!("New service nwc: {}!", service_nwc.user_pubkey());
    // notify new key
    let keys = state.pubkeys.lock().unwrap();
    let nwc = service_nwc.nwc_uri();
    keys.send_if_modified(|current| {
        if current.contains(&nwc.public_key) {
            false
        } else {
            current.push(nwc.public_key);
            true
        }
    });

    Ok(nwc)
}

pub async fn get_service_nwc(
    Extension(state): Extension<State>,
    Json(payload): Json<GetServiceNwcRequest>,
) -> Result<Json<String>, (StatusCode, String)> {
    match get_service_nwc_impl(payload.user_pubkey, payload.service_name, &state) {
        Ok(nwc) => Ok(Json(nwc.to_string())),
        Err(e) => Err(handle_anyhow_error(e)),
    }
}
