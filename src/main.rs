use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::{Method, StatusCode, Uri};
use axum::routing::post;
use axum::{http, Extension, Router};
use clap::Parser;
use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use nostr::key::XOnlyPublicKey;
use tokio::sync::watch;
use tokio::sync::watch::Sender;
use tower_http::cors::{Any, CorsLayer};

use crate::config::*;
use crate::models::service_nwc::ServiceNwc;
use crate::models::user_nwc::UserNwc;
use crate::models::MIGRATIONS;
use crate::routes::*;

mod config;
mod models;
mod routes;
mod subscriber;

#[derive(Clone)]
pub struct State {
    pubkeys: Arc<Mutex<Sender<Vec<XOnlyPublicKey>>>>,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config: Config = Config::parse();

    // Create the datadir if it doesn't exist
    let path = PathBuf::from(&config.data_dir);
    std::fs::create_dir_all(path.clone())?;

    let db_path = {
        let mut path = path.clone();
        path.push("db.sqlite");
        path
    };

    // DB management
    let manager = ConnectionManager::<SqliteConnection>::new(db_path.to_str().unwrap());
    let db_pool = Pool::builder()
        .max_size(16)
        .connection_customizer(Box::new(ConnectionOptions {
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(30)),
        }))
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool");

    let start = {
        let connection = &mut db_pool.get()?;
        // run migrations if needed
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("migrations could not run");

        let service_keys = ServiceNwc::get_all_keys(connection)?;
        let user_keys = UserNwc::get_all_keys(connection)?;

        let mut keys = Vec::with_capacity(service_keys.len() + user_keys.len());
        keys.extend(service_keys);
        keys.extend(user_keys);
        keys
    };

    let (tx, rx) = watch::channel(start);

    let tx_shared = Arc::new(Mutex::new(tx));

    let state = State {
        db_pool,
        pubkeys: tx_shared.clone(),
    };

    let addr: std::net::SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .expect("Failed to parse bind/port for webserver");

    println!("Webserver running on http://{}", addr);

    let server_router = Router::new()
        .route("/set-user-nwc", post(set_user_nwc))
        .route("/get-service-nwc", post(get_service_nwc))
        .fallback(fallback)
        .layer(Extension(state.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(vec![http::header::CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST]),
        );

    let server = axum::Server::bind(&addr).serve(server_router.into_make_service());

    tokio::spawn(subscriber::start_subscription(state.db_pool, rx));

    let graceful = server.with_graceful_shutdown(async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to create Ctrl+C shutdown signal");
    });

    // Await the server to receive the shutdown signal
    if let Err(e) = graceful.await {
        eprintln!("shutdown error: {}", e);
    }

    Ok(())
}

#[derive(Debug)]
pub struct ConnectionOptions {
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
    for ConnectionOptions
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route for {}", uri))
}
