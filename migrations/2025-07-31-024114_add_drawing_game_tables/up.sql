-- users table (oauth foundation)
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    external_id VARCHAR NOT NULL,
    provider VARCHAR NOT NULL,
    email VARCHAR,
    username VARCHAR,
    display_name VARCHAR,
    avatar_url VARCHAR,

    -- drawing specific stuff
    preferred_brush_color VARCHAR(7) DEFAULT '#000000',
    preferred_brush_size INT DEFAULT 5,
    drawing_privacy VARCHAR(20) DEFAULT 'public', -- 'public', 'friends', 'private'

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(external_id, provider)
);

-- canvas rooms for drawing games
CREATE TABLE canvas_rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_code VARCHAR(8) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_by INT REFERENCES users(id),
    max_players INT DEFAULT 8,
    is_private BOOLEAN DEFAULT false,
    game_mode VARCHAR(50) DEFAULT 'freeplay',  -- 'freeplay', 'guessing_game', 'teams'
    settings JSONB DEFAULT '{}', -- room specific settings (canvas size, time limits, etc.)
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_activity TIMESTAMP DEFAULT NOW()
);

-- game sessions within rooms
CREATE TABLE game_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES canvas_rooms(id) ON DELETE CASCADE,
    session_type VARCHAR(50) NOT NULL, -- 'quick_draw', 'team_battle', 'free_play'
    status VARCHAR(20) DEFAULT 'waiting', -- 'waiting', 'active', 'finished'
    current_round INT DEFAULT 1,
    max_rounds INT DEFAULT 3,
    round_time_limit INT DEFAULT 60, -- seconds
    started_at TIMESTAMP,
    finished_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW()
);

-- players in rooms (join/leave tracking)
CREATE TABLE room_players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES canvas_rooms(id) ON DELETE CASCADE,
    user_id INT NOT NULL REFERENCES users(id),
    joined_at TIMESTAMP DEFAULT NOW(),
    left_at TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    role VARCHAR(20) DEFAULT 'player', -- 'player', 'spectator', 'host'

    UNIQUE(room_id, user_id, joined_at)
);

-- simple teams
CREATE TABLE game_teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    color VARCHAR(7) DEFAULT '#5da893',
    score INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT NOW()
);

-- team membership
CREATE TABLE team_players (
    team_id UUID NOT NULL REFERENCES game_teams(id) ON DELETE CASCADE,
    user_id INT NOT NULL REFERENCES users(id),
    joined_at TIMESTAMP DEFAULT NOW(),

    PRIMARY KEY (team_id, user_id)
);


-- canvas persistence (store completed drawings)
CREATE TABLE saved_canvases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID REFERENCES canvas_rooms(id),
    session_id UUID REFERENCES game_sessions(id),
    created_by INT NOT NULL REFERENCES users(id),
    title VARCHAR(255),
    description TEXT,
    canvas_data JSONB NOT NULL, -- store the canvas operations/strokes
    thumbnail_url VARCHAR,
    is_public BOOLEAN DEFAULT true,
    likes_count INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT NOW()
);


-- basic user stats (will expand later)
CREATE TABLE user_game_stats (
    user_id INT PRIMARY KEY REFERENCES users(id),
    games_played INT DEFAULT 0,
    rooms_created INT DEFAULT 0,
    total_drawing_time INT DEFAULT 0, -- seconds
    canvases_saved INT DEFAULT 0,
    favorite_game_mode VARCHAR(50),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- indexes for perfffff
CREATE INDEX idx_users_external_provider ON users(external_id, provider);
CREATE INDEX idx_canvas_rooms_code ON canvas_rooms(room_code);
CREATE INDEX idx_canvas_rooms_active ON canvas_rooms(last_activity);
CREATE INDEX idx_room_players_active ON room_players(room_id, is_active) WHERE is_active = true;
CREATE INDEX idx_game_sessions_status ON game_sessions(status, created_at);
CREATE INDEX idx_saved_canvases_public ON saved_canvases(is_public, created_at) WHERE is_public = true;
CREATE INDEX idx_saved_canvases_user ON saved_canvases(created_by, created_at);

-- helper function to generate room codes
CREATE OR REPLACE FUNCTION generate_room_code() RETURNS VARCHAR(8) AS $$
DECLARE
    chars TEXT := 'ABCDEFGHIJKLMNPQRSTUVWXYZ123456789'; -- 0 and O not allow
    result TEXT := '';
    i INT;
    code TEXT;
    exists_check INT;
BEGIN
    LOOP
        result := '';
        FOR i IN 1..6 LOOP
            result := result || substr(chars, (random() * length(chars))::INT + 1, 1);
        END LOOP;
        code := 'DR' || result; -- prefix for "draw"

        SELECT COUNT(*) INTO exists_check FROM canvas_rooms WHERE room_code = code;

        IF exists_check = 0 THEN
            RETURN code;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- trigger to auto-gen room codes
CREATE OR REPLACE FUNCTION set_room_code() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.room_code IS NULL OR NEW.room_code = '' THEN
        NEW.room_code := generate_room_code();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_room_code
    BEFORE INSERT ON canvas_rooms
    FOR EACH ROW
    EXECUTE FUNCTION set_room_code();

-- trigger to update updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_canvas_rooms_updated_at
    BEFORE UPDATE ON canvas_rooms
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_user_game_stats_updated_at
    BEFORE UPDATE ON user_game_stats
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
