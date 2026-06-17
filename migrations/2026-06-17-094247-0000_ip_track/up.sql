
CREATE TABLE "ip_infos" (
	"ip" INET NOT NULL,
	"accounts_created" SMALLINT NOT NULL DEFAULT 0,
	"account_creation_count_reset" TIMESTAMP NOT NULL,
	"login_attempts" SMALLINT NOT NULL DEFAULT 0,
	"login_attempts_reset" TIMESTAMP NOT NULL,
	PRIMARY KEY("ip")
);
