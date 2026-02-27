CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL UNIQUE,
    display_name VARCHAR(20) UNIQUE,
    bio TEXT,
    birth_date DATE,
    profile_photo_url TEXT,
    kinks JSONB NOT NULL DEFAULT '[]',
    onboarding_complete BOOLEAN NOT NULL DEFAULT FALSE,
    moderation_status VARCHAR(20) NOT NULL DEFAULT 'clean',
    total_likes INTEGER NOT NULL DEFAULT 0,
    country VARCHAR(3),
    is_online BOOLEAN NOT NULL DEFAULT FALSE,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE follows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    following_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(follower_id, following_id)
);

CREATE TABLE likes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    liker_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    liked_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    match_session_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_profiles_credential ON profiles(credential_id);
CREATE INDEX idx_profiles_display_name ON profiles(display_name);
CREATE INDEX idx_follows_follower ON follows(follower_id);
CREATE INDEX idx_follows_following ON follows(following_id);
CREATE INDEX idx_likes_liker ON likes(liker_id);
CREATE INDEX idx_likes_liked ON likes(liked_id);
