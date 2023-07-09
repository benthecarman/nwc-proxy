// @generated automatically by Diesel CLI.

diesel::table! {
    service_nwc (request_key) {
        request_key -> Text,
        response_key -> Text,
        relay_url -> Text,
        service_name -> Text,
        user_pubkey -> Text,
        date_created -> Timestamp,
    }
}

diesel::table! {
    user_nwc (request_key) {
        request_key -> Text,
        response_key -> Text,
        relay_url -> Text,
        user_pubkey -> Text,
        date_created -> Timestamp,
    }
}

diesel::table! {
    users (pubkey) {
        pubkey -> Text,
        date_created -> Timestamp,
    }
}

diesel::joinable!(service_nwc -> users (user_pubkey));
diesel::joinable!(user_nwc -> users (user_pubkey));

diesel::allow_tables_to_appear_in_same_query!(service_nwc, user_nwc, users,);
