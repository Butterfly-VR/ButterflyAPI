DROP TABLE "group_members" CASCADE;
DROP TABLE "groups" CASCADE;
DROP TABLE "friends" CASCADE;
DROP TABLE "tags" CASCADE;
DROP TABLE "chat_session_messages" CASCADE;
DROP TABLE "chat_session_members" CASCADE;
DROP TABLE "tokens" CASCADE;
DROP TABLE "unverified_users" CASCADE;
DROP TABLE "user_reports" CASCADE;
DROP TABLE "ip_addresses" CASCADE;
DROP TABLE "notifications" CASCADE;
DROP TABLE "moderations" CASCADE;
DROP TABLE "object_download_tokens" CASCADE;
DROP TABLE "user_ips" CASCADE;
DROP TABLE "ip_infos" CASCADE;
DROP TABLE "users" CASCADE;
DROP TABLE "instances" CASCADE;
DROP TABLE "objects" CASCADE;
DROP TABLE "licenses" CASCADE;

-- Lookup tables
DROP TABLE "group_publicities" CASCADE;
DROP TABLE "report_types" CASCADE;
DROP TABLE "report_target_types" CASCADE;
DROP TABLE "notification_types" CASCADE;
DROP TABLE "moderation_types" CASCADE;
DROP TABLE "profile_styles" CASCADE;
DROP TABLE "user_statuses" CASCADE;
DROP TABLE "object_publicities" CASCADE;
DROP TABLE "instance_publicities" CASCADE;
DROP TABLE "permissions_levels" CASCADE;
DROP TABLE "object_types" CASCADE;

DROP EXTENSION "pg_trgm";
