-- drop triggers first
DROP TRIGGER IF EXISTS trigger_user_game_stats_updated_at ON user_game_stats;
DROP TRIGGER IF EXISTS trigger_canvas_rooms_updated_at ON canvas_rooms;
DROP TRIGGER IF EXISTS trigger_users_updated_at ON users;
DROP TRIGGER IF EXISTS trigger_set_room_code ON canvas_rooms;

-- drop functions
DROP FUNCTION IF EXISTS update_updated_at();
DROP FUNCTION IF EXISTS set_room_code();
DROP FUNCTION IF EXISTS generate_room_code();

-- drop indexes
DROP INDEX IF EXISTS idx_saved_canvases_user;
DROP INDEX IF EXISTS idx_saved_canvases_public;
DROP INDEX IF EXISTS idx_game_sessions_status;
DROP INDEX IF EXISTS idx_room_players_active;
DROP INDEX IF EXISTS idx_canvas_rooms_active;
DROP INDEX IF EXISTS idx_canvas_rooms_code;
DROP INDEX IF EXISTS idx_users_external_provider;

-- drop tables in reverse dependency order
DROP TABLE IF EXISTS user_game_stats;
DROP TABLE IF EXISTS saved_canvases;
DROP TABLE IF EXISTS team_players;
DROP TABLE IF EXISTS game_teams;
DROP TABLE IF EXISTS room_players;
DROP TABLE IF EXISTS game_sessions;
DROP TABLE IF EXISTS canvas_rooms;
DROP TABLE IF EXISTS users;
