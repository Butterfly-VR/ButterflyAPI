CREATE EXTENSION IF NOT EXISTS "pg_trgm";

CREATE TABLE IF NOT EXISTS "users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL UNIQUE,
	"email" VARCHAR(128) NOT NULL UNIQUE,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"permissions_level" SMALLINT NOT NULL,
	"trust" INTEGER NOT NULL,
	"homeworld" UUID,
	"avatar" UUID,
	"instance" UUID,
	"identifier" BYTEA,
	"created_at" TIMESTAMP NOT NULL,
	"deleted_at" TIMESTAMP,
	"can_login" BOOLEAN NOT NULL DEFAULT true,
	"is_deactivated" BOOLEAN NOT NULL DEFAULT false,
	"upload_quota_used" BIGINT NOT NULL,
	"download_quota_used" BIGINT NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "users_username_trgm_index" ON "users" USING GIN ("username" gin_trgm_ops);
CREATE INDEX "users_email_hash_index" ON "users" USING HASH("email");
CREATE INDEX "users_instance_index" ON "users" ("instance");
CREATE INDEX "users_identifier_index" ON "users" ("identifier");

CREATE TABLE IF NOT EXISTS "moderations" (
	"id" UUID NOT NULL UNIQUE,
	"target" UUID NOT NULL,
	"moderator" UUID,
	"type" SMALLINT NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	"expires" TIMESTAMP,
	"details" TEXT,
	PRIMARY KEY("id")
);

CREATE INDEX "moderations_target_index" ON "moderations" ("target");
CREATE INDEX "moderations_moderator_index" ON "moderations" ("moderator");
CREATE INDEX "moderations_expires_index" ON "moderations" ("expires");

CREATE TABLE IF NOT EXISTS "notifications" (
	"id" UUID NOT NULL UNIQUE,
	"target" UUID,
	"type" SMALLINT NOT NULL,
	"header" VARCHAR(128),
	"body" TEXT,
	"additional_data" JSONB,
	"dismissed" BOOLEAN NOT NULL DEFAULT false,
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	"expires" TIMESTAMP,
	PRIMARY KEY("id")
);

CREATE INDEX "notifications_target_index" ON "notifications" ("target");
CREATE INDEX "notifications_expires_index" ON "notifications" ("expires");

CREATE TABLE IF NOT EXISTS "ip_addresses" (
	"user" UUID NOT NULL,
	"ip" INET NOT NULL,
	"first_seen" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("user", "ip")
);

CREATE INDEX "ip_addresses_user_index" ON "ip_addresses" ("user");
CREATE INDEX "ip_addresses_ip_index" ON "ip_addresses" ("ip");

CREATE TABLE IF NOT EXISTS "user_reports" (
	"id" UUID NOT NULL UNIQUE,
	"reporter" UUID NOT NULL,
	"target" UUID NOT NULL,
	"target_type" SMALLINT NOT NULL,
	"report_type" SMALLINT NOT NULL,
	"details" VARCHAR(4096) NOT NULL,
	"additional_data" JSONB,
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("id")
);

CREATE INDEX "user_reports_reporter_index" ON "user_reports" ("reporter");
CREATE INDEX "user_reports_target_index" ON "user_reports" ("target");

CREATE TABLE IF NOT EXISTS "unverified_users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL UNIQUE,
	"email" VARCHAR(128) NOT NULL UNIQUE,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"token" BYTEA NOT NULL,
	"expiry" TIMESTAMP NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "tokens" (
	"token" BYTEA NOT NULL UNIQUE,
	"user" UUID NOT NULL,
	"renewable" BOOLEAN NOT NULL,
	"expiry" TIMESTAMP NOT NULL DEFAULT now(),
	"last_used" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("token")
);

CREATE INDEX "tokens_user_index" ON "tokens" ("user");
CREATE INDEX "tokens_expiry_index" ON "tokens" ("expiry");

CREATE TABLE IF NOT EXISTS "chat_session_members" (
	"session" UUID NOT NULL,
	"user" UUID NOT NULL,
	"last_seen_message" UUID,
	"joined_at" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("session", "user")
);

CREATE INDEX "chat_session_members_session_index" ON "chat_session_members" ("session");
CREATE INDEX "chat_session_members_user_index" ON "chat_session_members" ("user");

CREATE TABLE IF NOT EXISTS "chat_session_messages" (
	"id" UUID NOT NULL UNIQUE,
	"session" UUID NOT NULL,
	"user" UUID,
	"content" VARCHAR(4096) NOT NULL,
	"sent_at" TIMESTAMP NOT NULL DEFAULT now(),
	"modified_at" TIMESTAMP,
	PRIMARY KEY("id")
);

CREATE INDEX "chat_session_messages_session_index" ON "chat_session_messages" ("session");
CREATE INDEX "chat_session_messages_user_index" ON "chat_session_messages" ("user");

CREATE TABLE IF NOT EXISTS "objects" (
	"id" UUID NOT NULL UNIQUE,
	"name" VARCHAR(32) NOT NULL UNIQUE,
	"description" VARCHAR(4096) NOT NULL,
	"flags" BOOLEAN[] NOT NULL,
	"updated_at" TIMESTAMP NOT NULL DEFAULT now(),
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	"verified" BOOLEAN NOT NULL,
	"object_size" BIGINT NOT NULL,
	"image_size" BIGINT NOT NULL,
	"creator" UUID NOT NULL,
	"object_type" SMALLINT NOT NULL,
	"likes" INTEGER NOT NULL,
	"dislikes" INTEGER NOT NULL,
	"publicity" SMALLINT NOT NULL,
	"license" UUID NOT NULL,
	"encryption_key" BYTEA NOT NULL,
	"encryption_iv" BYTEA NOT NULL,
	"deleted_at" TIMESTAMP,
	PRIMARY KEY("id")
);

CREATE INDEX "objects_name_trgm_index" ON "objects" USING GIN ("name" gin_trgm_ops);
CREATE INDEX "objects_updated_at_index" ON "objects" ("updated_at");
CREATE INDEX "objects_created_at_index" ON "objects" ("created_at");
CREATE INDEX "objects_object_size_index" ON "objects" ("object_size");
CREATE INDEX "objects_creator_index" ON "objects" ("creator");
CREATE INDEX "objects_likes_index" ON "objects" ("likes");
CREATE INDEX "objects_dislikes_index" ON "objects" ("dislikes");
CREATE INDEX "objects_license_index" ON "objects" ("license");

CREATE TABLE IF NOT EXISTS "licenses" (
	"id" UUID NOT NULL UNIQUE,
	"text" TEXT NOT NULL UNIQUE,
	PRIMARY KEY("id")
);

CREATE INDEX "licenses_text_hash_index" ON "licenses" USING HASH ("text");

CREATE TABLE IF NOT EXISTS "tags" (
	"tag" VARCHAR(32) NOT NULL,
	"object" UUID NOT NULL,
	PRIMARY KEY("tag", "object")
);

CREATE INDEX "tags_tag_index" ON "tags" ("tag");
CREATE INDEX "tags_object_index" ON "tags" ("object");

CREATE TABLE IF NOT EXISTS "instances" (
    "id" UUID NOT NULL UNIQUE,
	"server_token" BYTEA NOT NULL UNIQUE,
	"world" UUID NOT NULL,
	"name" VARCHAR(32) NOT NULL,
	"max_players" SMALLINT NOT NULL,
	"publicity" SMALLINT NOT NULL,
	"anyone_can_invite" BOOLEAN NOT NULL,
	"is_gameserver" BOOLEAN NOT NULL,
	"ip" INET NOT NULL,
	"port" INTEGER NOT NULL,
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("id")
);

CREATE INDEX "instances_name_trgm_index" ON "instances" USING GIN ("name" gin_trgm_ops);
CREATE INDEX "instances_server_token_index" ON "instances" ("server_token");
CREATE INDEX "instances_world_index" ON "instances" ("world");

ALTER TABLE "users"
ADD CONSTRAINT fk_users_objects_homeworlds FOREIGN KEY("homeworld") REFERENCES "objects"("id")
ON DELETE SET NULL;
ALTER TABLE "users"
ADD CONSTRAINT fk_users_objects_avatars FOREIGN KEY("avatar") REFERENCES "objects"("id")
ON DELETE SET NULL;
ALTER TABLE "users"
ADD CONSTRAINT fk_users_instances_instances FOREIGN KEY("instance") REFERENCES "instances"("id")
ON DELETE SET NULL;

ALTER TABLE "moderations"
ADD CONSTRAINT fk_moderations_users_targets FOREIGN KEY("target") REFERENCES "users"("id")
ON DELETE RESTRICT;
ALTER TABLE "moderations"
ADD CONSTRAINT fk_moderations_users_moderators FOREIGN KEY("moderator") REFERENCES "users"("id")
ON DELETE SET NULL;

ALTER TABLE "notifications"
ADD CONSTRAINT fk_notifications_users_targets FOREIGN KEY("target") REFERENCES "users"("id")
ON DELETE CASCADE;

ALTER TABLE "ip_addresses"
ADD CONSTRAINT fk_ip_addresses_users_users FOREIGN KEY("user") REFERENCES "users"("id")
ON DELETE CASCADE;

ALTER TABLE "user_reports"
ADD CONSTRAINT fk_user_reports_users_reporters FOREIGN KEY("reporter") REFERENCES "users"("id")
ON DELETE CASCADE;

ALTER TABLE "tokens"
ADD CONSTRAINT fk_tokens_users_users FOREIGN KEY("user") REFERENCES "users"("id")
ON DELETE CASCADE;

ALTER TABLE "chat_session_members"
ADD CONSTRAINT fk_chat_session_members_users_users FOREIGN KEY("user") REFERENCES "users"("id")
ON DELETE CASCADE;

ALTER TABLE "chat_session_messages"
ADD CONSTRAINT fk_chat_session_messages_chat_session_members_sessions_and_users
FOREIGN KEY("session", "user") REFERENCES "chat_session_members"("session", "user")
ON DELETE SET NULL (user);

ALTER TABLE "objects"
ADD CONSTRAINT fk_objects_users_creators FOREIGN KEY("creator") REFERENCES "users"("id")
ON DELETE RESTRICT;
ALTER TABLE "objects"
ADD CONSTRAINT fk_objects_licenses_licenses FOREIGN KEY("license") REFERENCES "licenses"("license")
ON DELETE RESTRICT;

ALTER TABLE "tags"
ADD CONSTRAINT fk_tags_objects_objects FOREIGN KEY("object") REFERENCES "objects"("id")
ON DELETE CASCADE;

ALTER TABLE "instances"
ADD CONSTRAINT fk_instances_objects_worlds FOREIGN KEY("world") REFERENCES "objects"("id")
ON DELETE CASCADE;
