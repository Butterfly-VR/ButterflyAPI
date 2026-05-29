CREATE TABLE IF NOT EXISTS "users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL UNIQUE,
	"email" VARCHAR(128) NOT NULL UNIQUE,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"permisions" BOOLEAN[] NOT NULL,
	"trust" INTEGER NOT NULL,
	"homeworld" UUID,
	"avatar" UUID,
	"instance" UUID,
	PRIMARY KEY("id")
);

CREATE INDEX "users_email_index_hash"
ON "users" USING HASH("email");

CREATE INDEX "users_trust_index" ON "users" ("trust");
CREATE INDEX "users_homeworld_index" ON "users" ("homeworld");
CREATE INDEX "users_avatar_index" ON "users" ("avatar");

CREATE INDEX "users_instance_index" ON "users" ("instance");

CREATE TABLE IF NOT EXISTS "unverified_users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL UNIQUE,
	"email" VARCHAR(128) NOT NULL UNIQUE,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"token" BYTEA NOT NULL,
	"expiry" TIMESTAMP NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "tokens" (
	"token" BYTEA NOT NULL UNIQUE,
	"user" UUID NOT NULL,
	"renewable" BOOLEAN NOT NULL,
	"expiry" TIMESTAMP NOT NULL DEFAULT now(),
	PRIMARY KEY("token")
);

CREATE INDEX "tokens_user_index" ON "tokens" ("user");

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
	"publicity" SMALLINT NOT NULL,
	"license" INTEGER NOT NULL,
	"encryption_key" BYTEA NOT NULL,
	"encryption_iv" BYTEA NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "objects_updated_at_index" ON "objects" ("updated_at");
CREATE INDEX "objects_created_at_index" ON "objects" ("created_at");
CREATE INDEX "objects_object_size_index" ON "objects" ("object_size");
CREATE INDEX "objects_creator_index" ON "objects" ("creator");
CREATE INDEX "objects_object_type_index" ON "objects" ("object_type");
CREATE INDEX "objects_publicity_index" ON "objects" ("publicity");

CREATE TABLE IF NOT EXISTS "licenses" (
	"license" SERIAL NOT NULL UNIQUE,
	"text" VARCHAR(100000) NOT NULL UNIQUE,
	PRIMARY KEY("license")
);

CREATE INDEX "licenses_text_index_hash"
ON "licenses" USING HASH ("text");

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
	"client_token" BYTEA NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "instances_world_index" ON "instances" ("world");
CREATE INDEX "instances_name_index" ON "instances" ("name");

ALTER TABLE "instances"
ADD CONSTRAINT fk_instances_objects FOREIGN KEY("world") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE "users"
ADD CONSTRAINT fk_users_instances FOREIGN KEY("instance") REFERENCES "instances"("id")
ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "users"
ADD CONSTRAINT fk_users_objects_homeworlds FOREIGN KEY("homeworld") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "users"
ADD CONSTRAINT fk_users_objects_avatars FOREIGN KEY("avatar") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE SET NULL;

ALTER TABLE "tokens"
ADD CONSTRAINT fk_tokens_users FOREIGN KEY("user") REFERENCES "users"("id")
ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE "objects"
ADD CONSTRAINT fk_objects_users FOREIGN KEY("creator") REFERENCES "users"("id")
ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "objects"
ADD CONSTRAINT fk_objects_licenses FOREIGN KEY("license") REFERENCES "licenses"("license")
ON UPDATE CASCADE ON DELETE NO ACTION;

ALTER TABLE "tags"
ADD CONSTRAINT fk_tags_objects FOREIGN KEY("object") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE CASCADE;
