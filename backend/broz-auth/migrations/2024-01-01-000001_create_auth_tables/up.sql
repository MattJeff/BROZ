CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    is_banned BOOLEAN NOT NULL DEFAULT FALSE,
    ban_until TIMESTAMPTZ,
    role VARCHAR(20) NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE oauth_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    provider_uid VARCHAR(255) NOT NULL,
    access_token_enc TEXT,
    refresh_token_enc TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, provider_uid)
);

CREATE TABLE email_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    code VARCHAR(6) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE password_resets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    code VARCHAR(6) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    device_fingerprint VARCHAR(255),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_credentials_email ON credentials(email);
CREATE INDEX idx_refresh_tokens_credential ON refresh_tokens(credential_id);
CREATE INDEX idx_oauth_accounts_credential ON oauth_accounts(credential_id);
CREATE INDEX idx_email_verifications_credential ON email_verifications(credential_id);
CREATE INDEX idx_password_resets_credential ON password_resets(credential_id);
