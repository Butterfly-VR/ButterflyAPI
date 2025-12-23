-- Remove foreign keys
ALTER TABLE "users" DROP CONSTRAINT users_homeworld_fkey;
ALTER TABLE "users" DROP CONSTRAINT users_avatar_fkey;

ALTER TABLE "tokens" DROP CONSTRAINT tokens_user_fkey;

ALTER TABLE "objects" DROP CONSTRAINT objects_creator_fkey;

ALTER TABLE "tags" DROP CONSTRAINT tags_object_fkey;

-- Drop tables in reverse order
DROP TABLE "tags";
DROP TABLE "objects";
DROP TABLE "licenses";
DROP TABLE "tokens";
DROP TABLE "users";
