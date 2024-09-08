-- Your SQL goes here
CREATE TABLE `discord_guilds`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`guild_id` TEXT NOT NULL,
	`manager_user_id` TEXT NOT NULL
);

CREATE TABLE `users`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`discord_id` TEXT
);

CREATE TABLE `secret_keys`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`secret_key_hash` TEXT NOT NULL,
	`user_id` TEXT NOT NULL
);

