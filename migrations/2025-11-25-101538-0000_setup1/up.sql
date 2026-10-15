CREATE EXTENSION "pg_trgm";

CREATE TABLE "licenses" (
	"id" UUID NOT NULL UNIQUE,
	"text" TEXT NOT NULL UNIQUE,
	PRIMARY KEY("id")
);

CREATE INDEX "licenses_text_hash_index" ON "licenses" ("text");

CREATE TABLE "objects" (
	"id" UUID NOT NULL UNIQUE,
	"name" VARCHAR(32) NOT NULL UNIQUE,
	"description" VARCHAR(4096) NOT NULL,
	"flags" BOOLEAN[] NOT NULL,
	"updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"verified" BOOLEAN NOT NULL,
	"object_size" BIGINT NOT NULL,
	"image_size" BIGINT NOT NULL,
	"creator" UUID NOT NULL, -- FK to users added after users table exists (cyclical w/ homeworld/avatar)
	"object_type" SMALLINT NOT NULL,
	"likes" INTEGER NOT NULL,
	"dislikes" INTEGER NOT NULL,
	"publicity" SMALLINT NOT NULL,
	"license" UUID NOT NULL REFERENCES "licenses"("id"),
	"encryption_key" BYTEA NOT NULL,
	"encryption_iv" BYTEA NOT NULL,
	"delete_at" TIMESTAMPTZ,
	PRIMARY KEY("id")
);

CREATE INDEX "objects_name_trgm_index" ON "objects" USING GIN ("name" gin_trgm_ops);
CREATE INDEX "objects_description_trgm_index" ON "objects" USING GIN ("description" gin_trgm_ops);
CREATE INDEX "objects_updated_at_index" ON "objects" ("updated_at");
CREATE INDEX "objects_created_at_index" ON "objects" ("created_at");
CREATE INDEX "objects_object_size_index" ON "objects" ("object_size");
CREATE INDEX "objects_creator_index" ON "objects" ("creator");
CREATE INDEX "objects_likes_index" ON "objects" ("likes");
CREATE INDEX "objects_dislikes_index" ON "objects" ("dislikes");
CREATE INDEX "objects_license_index" ON "objects" ("license");
CREATE INDEX "objects_delete_at_index" ON "objects" ("delete_at");

CREATE TABLE "instances" (
    "id" UUID NOT NULL UNIQUE,
	"server_token" BYTEA NOT NULL UNIQUE,
	"world" UUID NOT NULL REFERENCES "objects"("id") ON DELETE CASCADE,
	"name" VARCHAR(32) NOT NULL,
	"max_players" SMALLINT NOT NULL,
	"publicity" SMALLINT NOT NULL,
	"anyone_can_invite" BOOLEAN NOT NULL,
	"is_gameserver" BOOLEAN NOT NULL,
	"ip" INET NOT NULL,
	"port" INTEGER NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY("id")
);

CREATE INDEX "instances_name_trgm_index" ON "instances" USING GIN ("name" gin_trgm_ops);
CREATE INDEX "instances_server_token_index" ON "instances" ("server_token");
CREATE INDEX "instances_world_index" ON "instances" ("world");
CREATE INDEX "instances_created_at_index" ON "instances" ("created_at");

CREATE TABLE "users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL UNIQUE,
	"email" VARCHAR(128) NOT NULL UNIQUE,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"permissions_level" SMALLINT NOT NULL,
	"trust" INTEGER NOT NULL,
	"homeworld" UUID REFERENCES "objects"("id") ON DELETE SET NULL,
	"avatar" UUID REFERENCES "objects"("id") ON DELETE SET NULL,
	"instance" UUID REFERENCES "instances"("id") ON DELETE SET NULL,
	"identifier" BYTEA,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"delete_at" TIMESTAMPTZ,
	"can_login" BOOLEAN NOT NULL DEFAULT true,
	"upload_quota_used" BIGINT NOT NULL,
	"download_quota_used" BIGINT NOT NULL,
	"bio" VARCHAR(4096),
	"status" SMALLINT NOT NULL,
	"status_message" VARCHAR(32),
	"pronouns" VARCHAR(32),
	"profile_main_colour" VARCHAR(6), -- RGB hex color "RRGGBB"
	"profile_secondary_colour" VARCHAR(6), -- RGB hex color "RRGGBB"
	"profile_dark_mode" BOOLEAN NOT NULL DEFAULT false,
	"profile_style" SMALLINT NOT NULL DEFAULT 0,
	PRIMARY KEY("id")
);

CREATE INDEX "users_username_trgm_index" ON "users" USING GIN ("username" gin_trgm_ops);
CREATE INDEX "users_instance_index" ON "users" ("instance");
CREATE INDEX "users_identifier_index" ON "users" ("identifier");
CREATE INDEX "users_created_at_index" ON "users" ("created_at");
CREATE INDEX "users_delete_at_index" ON "users" ("delete_at");

CREATE TABLE "ip_infos" (
	"ip" INET NOT NULL,
	"accounts_created" SMALLINT NOT NULL DEFAULT 0,
	"account_creation_count_reset" TIMESTAMPTZ NOT NULL,
	"login_attempts" SMALLINT NOT NULL DEFAULT 0,
	"login_attempts_reset" TIMESTAMPTZ NOT NULL,
	PRIMARY KEY("ip")
);

CREATE TABLE "user_ips" (
	"user_id" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
	"ip" INET NOT NULL REFERENCES "ip_infos"("ip"),
	PRIMARY KEY("user_id", "ip")
);

CREATE INDEX "idx_user_ips_user_id" ON "user_ips"("user_id");
CREATE INDEX "idx_user_ips_ip" ON "user_ips"("ip");

CREATE TABLE object_download_tokens (
    token UUID PRIMARY KEY,
    object_id UUID NOT NULL REFERENCES "objects"("id") ON DELETE CASCADE,
    current_step SMALLINT NOT NULL DEFAULT 0,
    expiry TIMESTAMPTZ NOT NULL
);

CREATE INDEX "idx_object_download_tokens_object_id" ON "object_download_tokens" ("object_id");
CREATE INDEX "idx_object_download_tokens_expiry" ON "object_download_tokens" ("expiry");

CREATE TABLE "moderations" (
	"id" UUID NOT NULL UNIQUE,
	"target" UUID NOT NULL REFERENCES "users"("id"),
	"moderator" UUID REFERENCES "users"("id") ON DELETE SET NULL,
	"type" SMALLINT NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"expires" TIMESTAMPTZ,
	"details" TEXT,
	PRIMARY KEY("id")
);

CREATE INDEX "moderations_target_index" ON "moderations" ("target");
CREATE INDEX "moderations_moderator_index" ON "moderations" ("moderator");
CREATE INDEX "moderations_expires_index" ON "moderations" ("expires");
CREATE INDEX "moderations_created_at_index" ON "moderations" ("created_at");

CREATE TABLE "notifications" (
	"id" UUID NOT NULL UNIQUE,
	"target" UUID REFERENCES "users"("id") ON DELETE CASCADE,
	"type" SMALLINT NOT NULL,
	"header" VARCHAR(128),
	"body" TEXT,
	"additional_data" JSONB,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"expires" TIMESTAMPTZ,
	PRIMARY KEY("id")
);

CREATE INDEX "notifications_target_index" ON "notifications" ("target");
CREATE INDEX "notifications_expires_index" ON "notifications" ("expires");
CREATE INDEX "notifications_created_at_index" ON "notifications" ("created_at");

CREATE TABLE "ip_addresses" (
	"user" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
	"ip" INET NOT NULL,
	"first_seen" TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY("user", "ip")
);

CREATE INDEX "ip_addresses_user_index" ON "ip_addresses" ("user");
CREATE INDEX "ip_addresses_ip_index" ON "ip_addresses" ("ip");

CREATE TABLE "user_reports" (
	"id" UUID NOT NULL UNIQUE,
	"reporter" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
	"target" UUID NOT NULL,
	"target_type" SMALLINT NOT NULL,
	"report_type" SMALLINT NOT NULL,
	"details" VARCHAR(4096) NOT NULL,
	"additional_data" JSONB,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"resolved" BOOLEAN NOT NULL DEFAULT false,
	PRIMARY KEY("id")
);

CREATE INDEX "user_reports_reporter_index" ON "user_reports" ("reporter");
CREATE INDEX "user_reports_target_index" ON "user_reports" ("target");
CREATE INDEX "user_reports_created_at_index" ON "user_reports" ("created_at");

CREATE TABLE "unverified_users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL UNIQUE,
	"email" VARCHAR(128) NOT NULL UNIQUE,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"token" BYTEA NOT NULL,
	"expires" TIMESTAMPTZ NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY("id")
);

CREATE INDEX "unverified_users_created_at_index" ON "unverified_users" ("created_at");

CREATE TABLE "tokens" (
	"token" BYTEA NOT NULL UNIQUE,
	"user" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
	"renewable" BOOLEAN NOT NULL,
	"expires" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"last_used" TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY("token")
);

CREATE INDEX "tokens_user_index" ON "tokens" ("user");
CREATE INDEX "tokens_expires_index" ON "tokens" ("expires");

CREATE TABLE "chat_session_members" (
	"session" UUID NOT NULL,
	"user" UUID NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
	"last_seen_message" UUID,
	"joined_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY("session", "user")
);

CREATE INDEX "chat_session_members_session_index" ON "chat_session_members" ("session");
CREATE INDEX "chat_session_members_user_index" ON "chat_session_members" ("user");

CREATE TABLE "chat_session_messages" (
	"id" UUID NOT NULL UNIQUE,
	"session" UUID NOT NULL,
	"user" UUID REFERENCES "users"("id") ON DELETE SET NULL,
	"content" VARCHAR(4096) NOT NULL,
	"sent_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"modified_at" TIMESTAMPTZ,
	"delete_at" TIMESTAMPTZ,
	PRIMARY KEY("id"),
	FOREIGN KEY("session", "user") REFERENCES "chat_session_members"("session", "user")
);

CREATE INDEX "chat_session_messages_session_index" ON "chat_session_messages" ("session");
CREATE INDEX "chat_session_messages_user_index" ON "chat_session_messages" ("user");
CREATE INDEX "chat_session_messages_delete_at_index" ON "chat_session_messages" ("delete_at");

CREATE TABLE "tags" (
	"tag" VARCHAR(32) NOT NULL,
	"object" UUID NOT NULL REFERENCES "objects"("id") ON DELETE CASCADE,
	PRIMARY KEY("tag", "object")
);

CREATE INDEX "tags_tag_index" ON "tags" ("tag");
CREATE INDEX "tags_object_index" ON "tags" ("object");

CREATE TABLE "friends" (
    "user_id" UUID NOT NULL,
    "friend_id" UUID NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY("user_id", "friend_id")
);

CREATE INDEX "idx_friends_user_id" ON "friends" ("user_id");
CREATE INDEX "idx_friends_friend_id" ON "friends" ("friend_id");
CREATE INDEX "idx_friends_created_at" ON "friends" ("created_at");

CREATE TABLE "groups"(
	"id" UUID NOT NULL UNIQUE,
	"name" VARCHAR(32) NOT NULL UNIQUE,
	"description" VARCHAR(4096) NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	"creator" UUID NOT NULL,
	"publicity" SMALLINT NOT NULL,
	"delete_at" TIMESTAMPTZ,
	PRIMARY KEY("id")
);

CREATE INDEX "idx_groups_name" ON "groups" USING GIN ("name" gin_trgm_ops);
CREATE INDEX "idx_groups_description" ON "groups" USING GIN ("description" gin_trgm_ops);
CREATE INDEX "idx_groups_creator" ON "groups" ("creator");
CREATE INDEX "idx_groups_created_at" ON "groups" ("created_at");
CREATE INDEX "idx_groups_delete_at" ON "groups" ("delete_at");

CREATE TABLE "group_members"(
	"group_id" UUID NOT NULL,
	"user_id" UUID NOT NULL,
	"created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY("group_id", "user_id")
);

CREATE INDEX "idx_group_members_group_id" ON "group_members" ("group_id");
CREATE INDEX "idx_group_members_user_id" ON "group_members" ("user_id");
CREATE INDEX "idx_group_members_created_at" ON "group_members" ("created_at");

-- Only remaining FK: breaks the users <-> objects cycle
-- (users.homeworld/avatar -> objects, objects.creator -> users)
ALTER TABLE "objects"
ADD CONSTRAINT fk_objects_users_creators FOREIGN KEY("creator") REFERENCES "users"("id");
