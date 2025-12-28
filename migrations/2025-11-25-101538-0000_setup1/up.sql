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
	PRIMARY KEY("id")
);

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

CREATE TABLE IF NOT EXISTS "objects" (
	"id" UUID NOT NULL UNIQUE,
	"name" VARCHAR(32) NOT NULL UNIQUE,
	"description" VARCHAR(4096) NOT NULL,
	"flags" BOOLEAN[] NOT NULL,
	"updated_at" TIMESTAMP NOT NULL DEFAULT now(),
	"created_at" TIMESTAMP NOT NULL DEFAULT now(),
	"verified" BOOLEAN NOT NULL,
	"object_size" INTEGER NOT NULL,
	"image_size" INTEGER NOT NULL,
	"creator" UUID NOT NULL,
	"object_type" SMALLINT NOT NULL,
	"publicity" SMALLINT NOT NULL,
	"license" INTEGER NOT NULL,
	"encryption_key" BYTEA NOT NULL,
	"encryption_iv" BYTEA NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "licenses" (
	"license" SERIAL NOT NULL UNIQUE,
	"text" VARCHAR(100000) NOT NULL UNIQUE,
	PRIMARY KEY("license")
);

CREATE INDEX "licenses_text_index"
ON "licenses" USING HASH ("text");

CREATE TABLE IF NOT EXISTS "tags" (
	"tag" VARCHAR(32) NOT NULL,
	"object" UUID NOT NULL,
	PRIMARY KEY("tag", "object")
);

CREATE INDEX "tags_tag_index"
ON "tags" ("tag");
CREATE INDEX "tags_object_index"
ON "tags" ("object");



ALTER TABLE "tokens"
ADD FOREIGN KEY("user") REFERENCES "users"("id")
ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE "objects"
ADD FOREIGN KEY("creator") REFERENCES "users"("id")
ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "objects"
ADD FOREIGN KEY("license") REFERENCES "licenses"("license")
ON UPDATE CASCADE ON DELETE NO ACTION;

ALTER TABLE "tags"
ADD FOREIGN KEY("object") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE "users"
ADD FOREIGN KEY("homeworld") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "users"
ADD FOREIGN KEY("avatar") REFERENCES "objects"("id")
ON UPDATE CASCADE ON DELETE SET NULL;
