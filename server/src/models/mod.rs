use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: String,
    pub discord_id: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub id: String,
    pub discord_id: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::discord_guilds)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DiscordGuild {
    pub id: String,
    pub guild_id: String,
    pub manager_user_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::discord_guilds)]
pub struct NewDiscordGuild {
    pub id: String,
    pub guild_id: String,
    pub manager_user_id: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::secret_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SecretKey {
    pub id: String,
    pub secret_key_hash: String,
    pub user_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::secret_keys)]
pub struct NewSecretKey {
    pub id: String,
    pub secret_key_hash: String,
    pub user_id: String,
}
