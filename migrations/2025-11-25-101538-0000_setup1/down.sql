-- Remove foreign keys
ALTER TABLE "instances"
DROP CONSTRAINT IF EXISTS "instances_world_fkey";

ALTER TABLE "users"
DROP CONSTRAINT IF EXISTS "users_instance_fkey";

ALTER TABLE "tokens"
DROP CONSTRAINT IF EXISTS "tokens_user_fkey";

ALTER TABLE "objects"
DROP CONSTRAINT IF EXISTS "objects_creator_fkey";
ALTER TABLE "objects"
DROP CONSTRAINT IF EXISTS "objects_license_fkey";

ALTER TABLE "tags"
DROP CONSTRAINT IF EXISTS "tags_object_fkey";

ALTER TABLE "users"
DROP CONSTRAINT IF EXISTS "users_homeworld_fkey";
ALTER TABLE "users"
DROP CONSTRAINT IF EXISTS "users_avatar_fkey";

-- Drop tables in reverse order
DROP TABLE "instances";
DROP TABLE "tags";
DROP TABLE "objects";
DROP TABLE "licenses";
DROP TABLE "tokens";
DROP TABLE "users";
DROP TABLE "unverified_users";
