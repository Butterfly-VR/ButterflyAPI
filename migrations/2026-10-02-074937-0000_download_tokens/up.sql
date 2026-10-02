CREATE TABLE object_download_tokens (
    token UUID PRIMARY KEY,
    object_id UUID NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX "idx_object_download_tokens_object_id" ON "object_download_tokens" (object_id);

ALTER TABLE "object_download_tokens"
    ADD CONSTRAINT "fk_object_download_tokens_object_id_objects"
    FOREIGN KEY (object_id)
    REFERENCES "objects"(id);
