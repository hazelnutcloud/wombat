use diesel::table;

table! {
  users {
    id -> Text,
    discord_id -> Nullable<Text>
  }
}

table! {
  secret_keys {
    id -> Text,
    secret_key_hash -> Text,
    user_id -> Text
  }
}

table! {
  discord_guilds {
    id -> Text,
    guild_id -> Text,
    manager_user_id -> Text
  }
}