CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE match_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_a_id UUID NOT NULL,
    user_b_id UUID NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    end_reason VARCHAR(50),
    duration_secs INTEGER DEFAULT 0
);

CREATE TABLE livecam_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    requester_id UUID NOT NULL,
    target_id UUID NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    room_id VARCHAR(100),
    expires_at TIMESTAMPTZ NOT NULL,
    responded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_match_sessions_users ON match_sessions(user_a_id, user_b_id);
CREATE INDEX idx_livecam_requests_target ON livecam_requests(target_id);
