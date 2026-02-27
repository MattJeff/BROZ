CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id UUID NOT NULL,
    reported_id UUID NOT NULL,
    report_type VARCHAR(50) NOT NULL,
    reason TEXT NOT NULL,
    context TEXT,
    match_session_id UUID,
    message_id UUID,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    reviewed_by UUID,
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sanctions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    report_id UUID REFERENCES reports(id),
    sanction_type VARCHAR(20) NOT NULL,
    reason TEXT NOT NULL,
    issued_by UUID NOT NULL,
    expires_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE admin_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id UUID NOT NULL,
    action VARCHAR(100) NOT NULL,
    target_user_id UUID,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reports_status ON reports(status);
CREATE INDEX idx_reports_reported ON reports(reported_id);
CREATE INDEX idx_sanctions_user ON sanctions(user_id);
CREATE INDEX idx_sanctions_active ON sanctions(user_id, is_active) WHERE is_active = TRUE;
CREATE INDEX idx_admin_actions_admin ON admin_actions(admin_id);
