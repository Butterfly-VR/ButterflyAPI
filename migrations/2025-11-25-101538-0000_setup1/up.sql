CREATE TABLE IF NOT EXISTS "users" (
	"id" UUID NOT NULL UNIQUE,
	"username" VARCHAR(32) NOT NULL,
	"email" TEXT NOT NULL,
	"password" BYTEA NOT NULL,
	"salt" BYTEA NOT NULL,
	"permisions" SMALLINT NOT NULL,
	"trust" INTEGER NOT NULL,
	"verified_email" BOOLEAN NOT NULL,
	"homeworld" UUID,
	"avatar" UUID,
	PRIMARY KEY("id")
);




CREATE TABLE IF NOT EXISTS "tokens" (
	"user" UUID NOT NULL,
	"token" BYTEA NOT NULL,
	"renewable" BOOLEAN NOT NULL,
	"expiry" TIMESTAMP,
	PRIMARY KEY("user", "token")
);


CREATE INDEX "tokens_index_0"
ON "tokens" ("user", "token");

CREATE TABLE IF NOT EXISTS "objects" (
	"id" UUID NOT NULL UNIQUE,
	"name" VARCHAR(32) NOT NULL,
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
	PRIMARY KEY("id")
);

CREATE TABLE IF NOT EXISTS "licenses" (
	"license" SERIAL NOT NULL,
	"text" VARCHAR(100000) NOT NULL UNIQUE,
	PRIMARY KEY("license")
);

CREATE TABLE IF NOT EXISTS "tags" (
	"tag" VARCHAR(32) NOT NULL,
	"object" UUID NOT NULL,
	PRIMARY KEY("tag", "object")
);


CREATE INDEX "tags_index_1"
ON "tags" ("tag", "object");
ALTER TABLE "tokens"
ADD FOREIGN KEY("user") REFERENCES "users"("id")
ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE "objects"
ADD FOREIGN KEY("creator") REFERENCES "users"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "objects"
ADD FOREIGN KEY("license") REFERENCES "licenses"("license")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "tags"
ADD FOREIGN KEY("object") REFERENCES "objects"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "users"
ADD FOREIGN KEY("homeworld") REFERENCES "objects"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "users"
ADD FOREIGN KEY("avatar") REFERENCES "objects"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
