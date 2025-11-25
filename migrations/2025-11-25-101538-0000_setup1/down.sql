-- ai generated
-- Remove foreign keys
ALTER TABLE "users" DROP CONSTRAINT IF EXISTS users_homeworld_fkey;
ALTER TABLE "users" DROP CONSTRAINT IF EXISTS users_avatar_fkey;

ALTER TABLE "tokens" DROP CONSTRAINT IF EXISTS tokens_user_fkey;

ALTER TABLE "objects" DROP CONSTRAINT IF EXISTS objects_creator_fkey;

ALTER TABLE "tags" DROP CONSTRAINT IF EXISTS tags_object_fkey;


-- Drop indexes
DROP INDEX IF EXISTS "tokens_index_0";
DROP INDEX IF EXISTS "tags_index_1";


-- Drop tables in reverse order
DROP TABLE IF EXISTS "tags";
DROP TABLE IF EXISTS "objects";
DROP TABLE IF EXISTS "tokens";
DROP TABLE IF EXISTS "users";


-- Drop ENUM types
DROP TYPE IF EXISTS "object_type";
DROP TYPE IF EXISTS "permision_level";
