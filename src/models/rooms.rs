use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanvasRoomView {
    pub id: Uuid,
    pub room_code: String,
    pub name: String,
    pub created_by: Option<i32>,
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
        use crate::models::User;
        use crate::models::GameTeam;
        use chrono::NaiveDateTime;
        use diesel::prelude::*;
        use diesel_async::{AsyncPgConnection, RunQueryDsl};

        #[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable, Associations)]
        #[diesel(belongs_to(User, foreign_key = created_by))]
        #[diesel(table_name = canvas_rooms)]
        pub struct CanvasRoom {
            pub id: Uuid,
            pub room_code: String,
            pub name: String,
            pub created_by: Option<i32>,
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

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Selectable, Associations)]
        #[diesel(belongs_to(CanvasRoom, foreign_key = room_id))]
        #[diesel(belongs_to(User, foreign_key = user_id))]
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

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, Associations)]
        #[diesel(belongs_to(CanvasRoom, foreign_key = room_id))]
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

            pub async fn find_by_code(room_code: &str, conn: &mut AsyncPgConnection) -> QueryResult<Option<CanvasRoom>> {
                canvas_rooms::table
                    .filter(canvas_rooms::room_code.eq(room_code))
                    .first(conn)
                    .await
                    .optional()
            }

            pub async fn active_player_count(&self, conn: &mut AsyncPgConnection) -> QueryResult<i64> {
                room_players::table
                    .filter(room_players::room_id.eq(self.id))
                    .filter(room_players::is_active.eq(true))
                    .count()
                    .get_result(conn)
                    .await
            }

            pub async fn has_active_player(&self, user_id: i32, conn: &mut AsyncPgConnection) -> QueryResult<bool> {
                let count: i64 = room_players::table
                    .filter(room_players::room_id.eq(self.id))
                    .filter(room_players::user_id.eq(user_id))
                    .filter(room_players::is_active.eq(true))
                    .count()
                    .get_result(conn)
                    .await?;
                Ok(count > 0)
            }

            pub async fn add_player(&self, user_id: i32, role: Option<String>, conn: &mut AsyncPgConnection) -> QueryResult<RoomPlayer> {
                let new_player = NewRoomPlayer {
                    room_id: self.id,
                    user_id,
                    role,
                };

                diesel::insert_into(room_players::table)
                    .values(&new_player)
                    .returning(room_players::all_columns)
                    .get_result(conn)
                    .await
            }

            pub async fn active_players(&self, conn: &mut AsyncPgConnection) -> QueryResult<Vec<RoomPlayer>> {
                RoomPlayer::belonging_to(self)
                    .filter(room_players::is_active.eq(true))
                    .load(conn)
                    .await
            }

            pub async fn current_session(&self, conn: &mut AsyncPgConnection) -> QueryResult<Option<GameSession>> {
                GameSession::belonging_to(self)
                    .filter(game_sessions::status.ne("finished"))
                    .order_by(game_sessions::created_at.desc())
                    .first(conn)
                    .await
                    .optional()
            }

            pub async fn list_public_with_player_counts(conn: &mut AsyncPgConnection) -> QueryResult<Vec<CanvasRoomView>> {
                let rooms = canvas_rooms::table
                    .filter(canvas_rooms::is_private.eq(false))
                    .order_by(canvas_rooms::created_at.desc())
                    .load::<CanvasRoom>(conn)
                    .await?;

                let mut room_views = Vec::new();
                for room in rooms {
                    let player_count = room.active_players(conn).await?.len();
                    room_views.push(CanvasRoomView {
                        player_count,
                        ..room.into()
                    });
                }

                Ok(room_views)
            }

            pub async fn get_with_details(&self, conn: &mut AsyncPgConnection) -> QueryResult<RoomWithPlayersView> {
                // Get active players with user data
                let players_data = room_players::table
                    .left_join(users::table.on(room_players::user_id.eq(users::id)))
                    .filter(room_players::room_id.eq(self.id))
                    .filter(room_players::is_active.eq(true))
                    .select((
                        RoomPlayer::as_select(),
                        users::id.nullable(),
                        users::username.nullable(),
                        users::display_name.nullable(),
                        users::avatar_url.nullable(),
                    ))
                    .load::<(RoomPlayer, Option<i32>, Option<String>, Option<String>, Option<String>)>(conn)
                    .await?;

                let players: Vec<RoomPlayerView> = players_data
                    .into_iter()
                    .map(|(player, user_id, username, display_name, avatar_url)| {
                        let mut player_view: RoomPlayerView = player.into();

                        // If we have a user_id, then we have user data (left join matched)
                        if let Some(id) = user_id {
                            player_view.user = Some(crate::models::users::UserView {
                                id,
                                external_id: "".to_string(),
                                provider: "".to_string(),
                                email: None,
                                username,
                                display_name,
                                avatar_url,
                                preferred_brush_color: None,
                                preferred_brush_size: None,
                                drawing_privacy: None,
                            });
                        }

                        player_view
                    })
                    .collect();

                // Get current session
                let current_session = self.current_session(conn).await?.map(|session| session.into());

                // Create room view with player count - clone self before converting
                let mut room_view: CanvasRoomView = (*self).clone().into();
                room_view.player_count = players.len();

                Ok(RoomWithPlayersView {
                    room: room_view,
                    players,
                    current_session,
                })
            }

            /// delete a room if the user is authorized (creator/host)
            /// returns the number of deleted rows (should be 1 if successful)
            pub async fn delete_room_with_auth_check(
                room_id: Uuid,
                user_id: i32,
                conn: &mut AsyncPgConnection
            ) -> Result<usize, RoomDeleteError> {
                use diesel::prelude::*;
                use diesel_async::RunQueryDsl;
                use crate::schema::canvas_rooms;

                let room: CanvasRoom = canvas_rooms::table
                    .find(room_id)
                    .first(conn)
                    .await
                    .map_err(|e| match e {
                        diesel::result::Error::NotFound => RoomDeleteError::RoomNotFound,
                        _ => RoomDeleteError::Database(e),
                    })?;

                if room.created_by != Some(user_id) {
                    return Err(RoomDeleteError::Unauthorized);
                }

                // delete room - CASCADE handles related records
                diesel::delete(canvas_rooms::table.find(room_id))
                    .execute(conn)
                    .await
                    .map_err(RoomDeleteError::Database)
            }

            pub async fn kick_player(
                host_id: i32,
                user_id: i32,
                room_id: Uuid,
                conn: &mut AsyncPgConnection
            ) -> Result<usize, KickPlayerError> {

                let room: CanvasRoom = canvas_rooms::table
                    .find(room_id)
                    .first(conn)
                    .await
                    .map_err(|e| match e {
                        diesel::result::Error::NotFound => KickPlayerError::RoomNotFound,
                        _ => KickPlayerError::Database(e),
                    })?;

                if room.created_by != Some(host_id) {
                    return Err(KickPlayerError::Unauthorized);
                }
                diesel::update(
                    room_players::table
                        .filter(room_players::user_id.eq(user_id))
                        .filter(room_players::room_id.eq(room_id))
                )
                .set((
                    room_players::is_active.eq(false),
                    room_players::left_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn)
                .await
                .map_err(KickPlayerError::Database)
            }

        }

        impl GameSession {
            pub fn is_active(&self) -> bool {
                self.status.as_deref() == Some("active")
            }

            pub fn is_finished(&self) -> bool {
                self.status.as_deref() == Some("finished")
            }

            pub async fn teams(&self, conn: &mut AsyncPgConnection) -> QueryResult<Vec<GameTeam>> {
                GameTeam::belonging_to(self).load(conn).await
            }

            pub async fn room(&self, conn: &mut AsyncPgConnection) -> QueryResult<CanvasRoom> {
                canvas_rooms::table.find(self.room_id).first(conn).await
            }
        }

        impl RoomPlayer {
            pub async fn user(&self, conn: &mut AsyncPgConnection) -> QueryResult<User> {
                users::table.find(self.user_id).first(conn).await
            }

            pub async fn room(&self, conn: &mut AsyncPgConnection) -> QueryResult<CanvasRoom> {
                canvas_rooms::table.find(self.room_id).first(conn).await
            }

            pub async fn leave_room(
                user_id: i32,
                room_id: Uuid,
                conn: &mut AsyncPgConnection
            ) -> QueryResult<usize> {
                diesel::update(
                    room_players::table
                        .filter(room_players::room_id.eq(room_id))
                        .filter(room_players::user_id.eq(user_id))
                        .filter(room_players::is_active.eq(true))
                )
                .set((
                    room_players::is_active.eq(false),
                    room_players::left_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn)
                .await
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
                    room_id: Uuid::new_v4(), // this will be set when creating
                    session_type: view.session_type,
                    max_rounds: view.max_rounds,
                    round_time_limit: view.round_time_limit,
                }
            }
        }

        #[derive(Debug)]
        pub enum RoomDeleteError {
            Database(diesel::result::Error),
            RoomNotFound,
            Unauthorized,
        }

        impl std::fmt::Display for RoomDeleteError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    RoomDeleteError::Database(e) => write!(f, "Database error: {e}"),
                    RoomDeleteError::RoomNotFound => write!(f, "Room not found"),
                    RoomDeleteError::Unauthorized => write!(f, "Only the host can delete this room"),
                }
            }
        }

        #[derive(Debug)]
        pub enum KickPlayerError {
            Database(diesel::result::Error),
            RoomNotFound,
            PlayerNotFound,
            Unauthorized,
        }

        impl std::fmt::Display for KickPlayerError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    KickPlayerError::Database(e) => write!(f, "Database error: {e}"),
                    KickPlayerError::RoomNotFound => write!(f, "Room not found"),
                    KickPlayerError::PlayerNotFound => write!(f, "Player not found"),
                    KickPlayerError::Unauthorized => write!(f, "Only the host can kick players"),
                }
            }
        }

        impl std::error::Error for RoomDeleteError {}
        impl std::error::Error for KickPlayerError {}
    }
}
