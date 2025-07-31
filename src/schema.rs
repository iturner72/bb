// @generated automatically by Diesel CLI.

diesel::table! {
    canvas_rooms (id) {
        id -> Uuid,
        #[max_length = 8]
        room_code -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        created_by -> Nullable<Int4>,
        max_players -> Nullable<Int4>,
        is_private -> Nullable<Bool>,
        #[max_length = 50]
        game_mode -> Nullable<Varchar>,
        settings -> Nullable<Jsonb>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        last_activity -> Nullable<Timestamp>,
    }
}

diesel::table! {
    game_sessions (id) {
        id -> Uuid,
        room_id -> Uuid,
        #[max_length = 50]
        session_type -> Varchar,
        #[max_length = 20]
        status -> Nullable<Varchar>,
        current_round -> Nullable<Int4>,
        max_rounds -> Nullable<Int4>,
        round_time_limit -> Nullable<Int4>,
        started_at -> Nullable<Timestamp>,
        finished_at -> Nullable<Timestamp>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    game_teams (id) {
        id -> Uuid,
        session_id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 7]
        color -> Nullable<Varchar>,
        score -> Nullable<Int4>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    room_players (id) {
        id -> Uuid,
        room_id -> Uuid,
        user_id -> Int4,
        joined_at -> Nullable<Timestamp>,
        left_at -> Nullable<Timestamp>,
        is_active -> Nullable<Bool>,
        #[max_length = 20]
        role -> Nullable<Varchar>,
    }
}

diesel::table! {
    saved_canvases (id) {
        id -> Uuid,
        room_id -> Nullable<Uuid>,
        session_id -> Nullable<Uuid>,
        created_by -> Int4,
        #[max_length = 255]
        title -> Nullable<Varchar>,
        description -> Nullable<Text>,
        canvas_data -> Jsonb,
        thumbnail_url -> Nullable<Varchar>,
        is_public -> Nullable<Bool>,
        likes_count -> Nullable<Int4>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    team_players (team_id, user_id) {
        team_id -> Uuid,
        user_id -> Int4,
        joined_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    user_game_stats (user_id) {
        user_id -> Int4,
        games_played -> Nullable<Int4>,
        rooms_created -> Nullable<Int4>,
        total_drawing_time -> Nullable<Int4>,
        canvases_saved -> Nullable<Int4>,
        #[max_length = 50]
        favorite_game_mode -> Nullable<Varchar>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        external_id -> Varchar,
        provider -> Varchar,
        email -> Nullable<Varchar>,
        username -> Nullable<Varchar>,
        display_name -> Nullable<Varchar>,
        avatar_url -> Nullable<Varchar>,
        #[max_length = 7]
        preferred_brush_color -> Nullable<Varchar>,
        preferred_brush_size -> Nullable<Int4>,
        #[max_length = 20]
        drawing_privacy -> Nullable<Varchar>,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(canvas_rooms -> users (created_by));
diesel::joinable!(game_sessions -> canvas_rooms (room_id));
diesel::joinable!(game_teams -> game_sessions (session_id));
diesel::joinable!(room_players -> canvas_rooms (room_id));
diesel::joinable!(room_players -> users (user_id));
diesel::joinable!(saved_canvases -> canvas_rooms (room_id));
diesel::joinable!(saved_canvases -> game_sessions (session_id));
diesel::joinable!(saved_canvases -> users (created_by));
diesel::joinable!(team_players -> game_teams (team_id));
diesel::joinable!(team_players -> users (user_id));
diesel::joinable!(user_game_stats -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    canvas_rooms,
    game_sessions,
    game_teams,
    room_players,
    saved_canvases,
    team_players,
    user_game_stats,
    users,
);
