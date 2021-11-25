table! {
    users (id) {
        id -> Bigint,
        discord_id -> Bigint,
        username -> Text,
        password -> Text,
        permissions -> Text,
        status -> Text,
        status_changed ->Bigint,
        discoverer -> Text,
        reviewer ->Text,
        properties ->Text,
        created ->Bigint,
    }
}
table! {
    auth_tokens (id) {
        id -> Bigint,
        user -> Bigint,
        token -> Text,
        created ->Bigint,
    }
}
table! {
    client_keys (id) {
        id -> Bigint,
        api_key -> Text,
        created ->Bigint,
    }
}
table! {
    settings (id) {
        id -> Bigint,
        setting -> Text,
        value ->Text,
        updated ->Bigint,

    }
}
table! {
    otps (id) {
        id -> Bigint,
        user -> Bigint,
        password ->Text,
        expiration ->Bigint,
        created ->Bigint,

    }
}
