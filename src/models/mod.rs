use diesel_migrations::{embed_migrations, EmbeddedMigrations};

pub mod schema;
pub mod service_nwc;
pub mod user;
pub mod user_nwc;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[cfg(test)]
mod test {
    use crate::models::service_nwc::ServiceNwc;
    use crate::models::user::*;
    use crate::models::user_nwc::UserNwc;
    use bitcoin::hashes::hex::ToHex;
    use bitcoin::secp256k1::rand::Rng;
    use bitcoin::secp256k1::{rand, PublicKey};
    use diesel::{Connection, SqliteConnection};
    use diesel_migrations::MigrationHarness;
    use nostr::nips::nip47::NostrWalletConnectURI;
    use std::str::FromStr;

    const PUB_KEY_STR: &str = "032e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af";
    const NWC_URI_STR: &str = "nostr+walletconnect://5fa11a95186e2bdc05e047d8573721b407aaa54e5c39f93b2811f176a65ac5f8?relay=wss%3A%2F%2Fnostr.mutinywallet.com%2F&secret=e0d196bf4af30401332085702d35ec0c0b6d6bcc43b76d05d9d9898b2c2c6d94";

    fn gen_tmp_db_name() -> String {
        let rng = rand::thread_rng();
        let rand_string: String = rng
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(30)
            .collect::<Vec<u8>>()
            .to_hex();
        format!("/tmp/nwc_proxy_{}.sqlite", rand_string)
    }

    fn create_database(db_name: &str) -> SqliteConnection {
        let mut connection = SqliteConnection::establish(db_name).unwrap();

        connection
            .run_pending_migrations(crate::models::MIGRATIONS)
            .expect("migrations could not run");

        connection
    }

    fn teardown_database(db_name: &str) {
        std::fs::remove_file(db_name).unwrap();
    }

    #[test]
    fn test_create_and_find_user() {
        let db_name = gen_tmp_db_name();
        let conn = &mut create_database(&db_name);

        let pk = PublicKey::from_str(PUB_KEY_STR).unwrap();

        // create user
        let user = User::create(conn, pk).unwrap();

        assert_eq!(user.pubkey(), pk.clone());

        // get user
        let found = User::find(conn, &pk).unwrap().unwrap();

        assert_eq!(user, found);

        teardown_database(&db_name);
    }

    #[test]
    fn test_user_nwc() {
        let db_name = gen_tmp_db_name();
        let conn = &mut create_database(&db_name);

        let pk = PublicKey::from_str(PUB_KEY_STR).unwrap();
        User::create(conn, pk).unwrap();

        let nwc = NostrWalletConnectURI::from_str(NWC_URI_STR).unwrap();
        let db = UserNwc::create(conn, nwc, pk).unwrap();

        let found = UserNwc::find_by_user(conn, &pk).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], db);

        teardown_database(&db_name);
    }

    #[test]
    fn test_service_nwc() {
        let db_name = gen_tmp_db_name();
        let conn = &mut create_database(&db_name);

        let pk = PublicKey::from_str(PUB_KEY_STR).unwrap();
        User::create(conn, pk).unwrap();

        let db = ServiceNwc::generate(pk, "service".to_string());
        ServiceNwc::insert(conn, &db).unwrap();

        let found = ServiceNwc::find_by_user(conn, &pk).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], db);

        teardown_database(&db_name);
    }
}
