use crate::models::service_nwc::{ServiceNwc, DEFAULT_SERVICE_RELAY};
use crate::models::user_nwc::UserNwc;
use anyhow::anyhow;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use nostr::key::XOnlyPublicKey;
use nostr::nips::nip47::{Method, NostrWalletConnectURI, Request, RequestParams};
use nostr::prelude::{decrypt, encrypt, Secp256k1};
use nostr::{Event, EventBuilder, Filter, Keys, Kind, Tag, Timestamp};
use nostr_sdk::{Client, RelayPoolNotification};
use std::time::Duration;
use tokio::sync::watch::Receiver;

pub async fn start_subscription(
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    mut rx: Receiver<Vec<XOnlyPublicKey>>,
) -> anyhow::Result<()> {
    let keys = Keys::generate();
    loop {
        let client = Client::new(&keys);

        let db_relays = {
            let db = &mut db_pool.get()?;
            let relays = UserNwc::get_relays(db)?;

            relays.into_iter().map(|r| (r, None)).collect::<Vec<_>>()
        };
        client.add_relays(db_relays).await?;
        client.add_relay(DEFAULT_SERVICE_RELAY, None).await?;
        client.connect().await;

        let keys: Vec<XOnlyPublicKey> = rx.borrow().clone();
        let authors: Vec<String> = keys.iter().map(|k| k.to_string()).collect();

        let kinds = vec![Kind::WalletConnectRequest, Kind::WalletConnectResponse];

        let subscription = Filter::new()
            .kinds(kinds.clone())
            .pubkeys(keys)
            .since(Timestamp::now());

        let subscription2 = Filter::new()
            .kinds(kinds)
            .authors(authors)
            .since(Timestamp::now());

        client.subscribe(vec![subscription, subscription2]).await;

        println!("Listening for nwc events...");

        let mut notifications = client.notifications();
        loop {
            tokio::select! {
                Ok(notification) = notifications.recv() => {
                    if let RelayPoolNotification::Event(_url, event) = notification {
                        match event.kind {
                            Kind::WalletConnectRequest => {
                                println!("Received request");
                                tokio::spawn({
                                    let db_pool = db_pool.clone();
                                    let client = client.clone();
                                    async move {
                                        let fut = handle_request(
                                            db_pool,
                                            &client,
                                            event,
                                        );

                                        match tokio::time::timeout(Duration::from_secs(30), fut).await {
                                            Ok(Ok(_)) => {}
                                            Ok(Err(e)) => eprintln!("Error: {e}"),
                                            Err(_) => eprintln!("Timeout"),
                                        }
                                    }
                                });
                            }
                            Kind::WalletConnectResponse => {
                                println!("Received response");
                                 tokio::spawn({
                                    let db_pool = db_pool.clone();
                                    let client = client.clone();
                                    async move {
                                        let fut = handle_response(
                                            db_pool,
                                            &client,
                                            event,
                                        );

                                        match tokio::time::timeout(Duration::from_secs(30), fut).await {
                                            Ok(Ok(_)) => {}
                                            Ok(Err(e)) => eprintln!("Error: {e}"),
                                            Err(_) => eprintln!("Timeout"),
                                        }
                                    }
                                });
                            }
                            kind => println!("Received even with invalid kind: {:?}", kind)
                        }
                    }
                }
                _ = rx.changed() => {
                    break;
                }
            }
        }

        client.disconnect().await?;
    }
}

async fn handle_request(
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    client: &Client,
    event: Event,
) -> anyhow::Result<Option<Event>> {
    debug_assert!(event.kind == Kind::WalletConnectRequest);
    let request_key = {
        let p_tag = event.tags.into_iter().find_map(|tag| {
            if let Tag::PubKey(p, _) = tag {
                Some(p)
            } else {
                None
            }
        });

        if let Some(p_tag) = p_tag {
            p_tag
        } else {
            return Err(anyhow!("No p tag found"));
        }
    };

    let db = &mut db_pool.get()?;
    let service_nwc: ServiceNwc = {
        let opt = ServiceNwc::find_by_request_key(db, &request_key)?;
        opt.ok_or(anyhow!("No service nwc found"))?
    };

    let response_key = service_nwc.response_key();

    let context = Secp256k1::new();
    if event.pubkey != response_key.x_only_public_key(&context).0 {
        return Err(anyhow!("Event pubkey does not match response key"));
    }

    let decrypted = decrypt(&response_key, &service_nwc.request_key(), &event.content)?;
    let req: Request = Request::from_json(decrypted)?;

    // only respond to pay invoice requests
    if req.method != Method::PayInvoice {
        return Ok(None);
    }

    // todo check spending conditions

    let user_nwc: UserNwc = {
        let vec = UserNwc::find_by_user(db, &service_nwc.user_pubkey())?;
        vec.first().cloned().ok_or(anyhow!("No user nwc found"))?
    };
    let nwc = user_nwc.nwc_uri();
    let fwd_event = create_nwc_request(&nwc, req.params.invoice);

    client
        .send_event_to(nwc.relay_url.to_string(), fwd_event.clone())
        .await?;

    println!("Sent event to {}", nwc.relay_url);

    Ok(Some(fwd_event))
}

async fn handle_response(
    _db_pool: Pool<ConnectionManager<SqliteConnection>>,
    _client: &Client,
    event: Event,
) -> anyhow::Result<Option<Event>> {
    debug_assert!(event.kind == Kind::WalletConnectResponse);

    // todo

    Ok(None)
}

fn create_nwc_request(nwc: &NostrWalletConnectURI, invoice: String) -> Event {
    let req = Request {
        method: Method::PayInvoice,
        params: RequestParams { invoice },
    };

    let encrypted = encrypt(&nwc.secret, &nwc.public_key, req.as_json()).unwrap();
    let p_tag = Tag::PubKey(nwc.public_key, None);

    EventBuilder::new(Kind::WalletConnectRequest, encrypted, &[p_tag])
        .to_event(&Keys::new(nwc.secret))
        .unwrap()
}
