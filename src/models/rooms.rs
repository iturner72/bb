use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanvasRoomView {
    pub id: Uuid,
    pub room_code: String,
    pub name: String,
    pub created_by: i32,
    pub max_players: Option<i32>,
    pub is_private: Option<bool>,
    pub game_mode: Option<String>,
    pub settings: Option<serde_json::Value>,
    pub player_count: usize, // calculated field
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateRoomView {
    pub name: String,
    pub max_players: Option<i32>,
    pub is_private: Option<bool>,
    pub game_mode: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JoinRoomView {
    pub room_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomPlayerView {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: i32,
    pub role: Option<String>,
    pub is_active: Option<bool>,
    // joined user info
    pub user: Option<crate::models::users::UserView>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomWithPlayersView {
    pub room: CanvasRoomView,
    pub players: Vec<RoomPlayerView>,
    pub current_session: Option<GameSessionView>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSessionView {
    pub id: Uuid,
    pub room_id: Uuid,
    pub session_type: String,
    pub status: Option<String>,
    pub current_round: Option<i32>,
    pub max_rounds: Option<i32>,
    pub round_time_limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateSessionView {
    pub session_type: String,
    pub max_rounds: Option<i32>,
    pub round_time_limit: Option<i32>,
}

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use crate::schema::*;
        use chrono::NaiveDateTime;
        use diesel::prelude::*;

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
        #[diesel(table_name = canvas_rooms)]
        pub struct CanvasRoom {
            pub id: Uuid,
            pub room_code: String,
            pub name: String,
            pub created_by: i32,
            pub max_players: Option<i32>,
            pub is_private: Option<bool>,
            pub game_mode: Option<String>,
            pub settings: Option<serde_json::Value>,
            pub created_at: Option<NaiveDateTime>,
            pub updated_at: Option<NaiveDateTime>,
            pub last_activity: Option<NaiveDateTime>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = canvas_rooms)]
        pub struct NewCanvasRoom {
            pub name: String,
            pub created_by: i32,
            pub max_players: Option<i32>,
            pub is_private: Option<bool>,
            pub game_mode: Option<String>,
            pub settings: Option<serde_json::Value>,
        }

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
        #[diesel(table_name = room_players)]
        pub struct RoomPlayer {
            pub id: Uuid,
            pub room_id: Uuid,
            pub user_id: i32,
            pub joined_at: Option<NaiveDateTime>,
            pub left_at: Option<NaiveDateTime>,
            pub is_active: Option<bool>,
            pub role: Option<String>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = room_players)]
        pub struct NewRoomPlayer {
            pub room_id: Uuid,
            pub user_id: i32,
            pub role: Option<String>,
        }

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
        #[diesel(table_name = game_sessions)]
        pub struct GameSession {
            pub id: Uuid,
            pub room_id: Uuid,
            pub session_type: String,
            pub status: Option<String>,
            pub current_round: Option<i32>,
            pub max_rounds: Option<i32>,
            pub round_time_limit: Option<i32>,
            pub started_at: Option<NaiveDateTime>,
            pub finished_at: Option<NaiveDateTime>,
            pub created_at: Option<NaiveDateTime>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = game_sessions)]
        pub struct NewGameSession {
            pub room_id: Uuid,
            pub session_type: String,
            pub max_rounds: Option<i32>,
            pub round_time_limit: Option<i32>,
        }

        impl CanvasRoom {
            pub fn is_full(&self, current_players: usize) -> bool {
                if let Some(max) = self.max_players {
                    current_players >= max as usize
                } else {
                    false
                }
            }

            pub fn can_join(&self, current_players: usize) -> bool {
                !self.is_full(current_players)
            }
        }

        impl GameSession {
            pub fn is_active(&self) -> bool {
                self.status.as_deref() == Some("active")
            }

            pub fn is_finished(&self) -> bool {
                self.status.as_deref() == Some("finished")
            }
        }

        impl From<CanvasRoom> for CanvasRoomView {
            fn from(room: CanvasRoom) -> Self {
                CanvasRoomView {
                    id: room.id,
                    room_code: room.room_code,
                    name: room.name,
                    created_by: room.created_by,
                    max_players: room.max_players,
                    is_private: room.is_private,
                    game_mode: room.game_mode,
                    settings: room.settings,
                    player_count: 0, // this is calculated when fetching
                }
            }
        }

        impl From<CreateRoomView> for NewCanvasRoom {
            fn from(view: CreateRoomView) -> Self {
                NewCanvasRoom {
                    name: view.name,
                    created_by: 0, // this will be set when creating
                    max_players: view.max_players,
                    is_private: view.is_private,
                    game_mode: view.game_mode,
                    settings: view.settings,
                }
            }
        }

        impl From<RoomPlayer> for RoomPlayerView {
            fn from(player: RoomPlayer) -> Self {
                RoomPlayerView {
                    id: player.id,
                    room_id: player.room_id,
                    user_id: player.user_id,
                    role: player.role,
                    is_active: player.is_active,
                    user: None, // this will be populated with a join
                }
            }
        }

        impl From<GameSession> for GameSessionView {
            fn from(session: GameSession) -> Self {
                GameSessionView {
                    id: session.id,
                    room_id: session.room_id,
                    session_type: session.session_type,
                    status: session.status,
                    current_round: session.current_round,
                    max_rounds: session.max_rounds,
                    round_time_limit: session.round_time_limit,
                }
            }
        }

        impl From<CreateSessionView> for NewGameSession {
            fn from(view: CreateSessionView) -> Self {
                NewGameSession {
                    room_id: Uuid::new_v4(), // This will be set when creating
                    session_type: view.session_type,
                    max_rounds: view.max_rounds,
                    round_time_limit: view.round_time_limit,
                }
            }
        }
    }
}
