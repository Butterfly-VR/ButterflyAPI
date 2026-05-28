ALTER TABLE instances DROP COLUMN "client_token" CASCADE;
ALTER TABLE instances ADD COLUMN "ip" INET NOT NULL;
ALTER TABLE instances ADD COLUMN "port" INTEGER NOT NULL;
ALTER TABLE users ADD COLUMN "identifier" BYTEA NULL;

CREATE INDEX "users_identifier_idx" ON "users" ("identifier");
