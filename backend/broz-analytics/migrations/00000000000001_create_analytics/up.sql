CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE analytics_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    event_type VARCHAR(255) NOT NULL,
    properties JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE daily_stats (
    date DATE NOT NULL,
    metric VARCHAR(100) NOT NULL,
    value BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (date, metric)
);

CREATE INDEX idx_analytics_events_type ON analytics_events(event_type);
CREATE INDEX idx_analytics_events_user ON analytics_events(user_id);
CREATE INDEX idx_analytics_events_created ON analytics_events(created_at);
CREATE INDEX idx_daily_stats_date ON daily_stats(date);
