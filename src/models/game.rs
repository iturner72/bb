use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameTeamView {
    pub id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub score: Option<i32>,
    pub players: Vec<TeamPlayerView>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateTeamView {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamPlayerView {
    pub team_id: Uuid,
    pub user_id: i32,
    pub user: Option<crate::models::users::UserView>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserGameStatsView {
    pub user_id: i32,
    pub games_played: Option<i32>,
    pub rooms_created: Option<i32>,
    pub total_drawing_time: Option<i32>,
    pub canvases_saved: Option<i32>,
    pub favorite_game_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedCanvasView {
    pub id: Uuid,
    pub room_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub created_by: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canvas_data: serde_json::Value,
    pub thumbnail_url: Option<String>,
    pub is_public: Option<bool>,
    pub likes_count: Option<i32>,
    // joined data
    pub creator: Option<crate::models::users::UserView>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateCanvasView {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canvas_data: serde_json::Value,
    pub is_public: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanvasGalleryView {
    pub canvases: Vec<SavedCanvasView>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use crate::schema::*;
        use chrono::NaiveDateTime;
        use diesel::prelude::*;

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
        #[diesel(table_name = game_teams)]
        pub struct GameTeam {
            pub id: Uuid,
            pub session_id: Uuid,
            pub name: String,
            pub color: Option<String>,
            pub score: Option<i32>,
            pub created_at: Option<NaiveDateTime>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = game_teams)]
        pub struct NewGameTeam {
            pub session_id: Uuid,
            pub name: String,
            pub color: Option<String>,
        }

        #[derive(Debug, Serialize, Deserialize, Queryable)]
        #[diesel(table_name = team_players)]
        pub struct TeamPlayer {
            pub team_id: Uuid,
            pub user_id: i32,
            pub joined_at: Option<NaiveDateTime>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = team_players)]
        pub struct NewTeamPlayer {
            pub team_id: Uuid,
            pub user_id: i32,
        }

        #[derive(Debug, Serialize, Deserialize, Queryable)]
        #[diesel(table_name = user_game_stats)]
        pub struct UserGameStats {
            pub user_id: i32,
            pub games_played: Option<i32>,
            pub rooms_created: Option<i32>,
            pub total_drawing_time: Option<i32>,
            pub canvases_saved: Option<i32>,
            pub favorite_game_mode: Option<String>,
            pub created_at: Option<NaiveDateTime>,
            pub updated_at: Option<NaiveDateTime>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = user_game_stats)]
        pub struct NewUserGameStats {
            pub user_id: i32,
        }

        #[derive(Debug, AsChangeset)]
        #[diesel(table_name = user_game_stats)]
        pub struct UpdateUserGameStats {
            pub games_played: Option<i32>,
            pub rooms_created: Option<i32>,
            pub total_drawing_time: Option<i32>, // seconds
            pub canvases_saved: Option<i32>,
            pub favorite_game_mode: Option<String>,
        }

        #[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
        #[diesel(table_name = saved_canvases)]
        pub struct SavedCanvas {
            pub id: Uuid,
            pub room_id: Option<Uuid>,
            pub session_id: Option<Uuid>,
            pub created_by: i32,
            pub title: Option<String>,
            pub description: Option<String>,
            pub canvas_data: serde_json::Value,
            pub thumbnail_url: Option<String>,
            pub is_public: Option<bool>,
            pub likes_count: Option<i32>,
            pub created_at: Option<NaiveDateTime>,
        }

        #[derive(Debug, Insertable)]
        #[diesel(table_name = saved_canvases)]
        pub struct NewSavedCanvas {
            pub room_id: Option<Uuid>,
            pub session_id: Option<Uuid>,
            pub created_by: i32,
            pub title: Option<String>,
            pub description: Option<String>,
            pub canvas_data: serde_json::Value,
            pub is_public: Option<bool>,
        }

        impl UserGameStats {
            pub fn increment_games_played(&mut self) {
                self.games_played = Some(self.games_played.unwrap_or(0) + 1);
            }

            pub fn increment_rooms_created(&mut self) {
                self.rooms_created = Some(self.rooms_created.unwrap_or(0) + 1);
            }

            pub fn add_drawing_tie(&mut self, seconds: i32) {
                self.total_drawing_time = Some(self.total_drawing_time.unwrap_or(0) + seconds);
            }

            pub fn increment_canvases_saved(&mut self) {
                self.canvases_saved = Some(self.canvases_saved.unwrap_or(0) + 1);
            }
        }

        impl SavedCanvas {
            pub fn can_be_edited_by(&self, user_id: i32) -> bool {
                self.created_by == user_id
            }

            pub fn increment_likes(&mut self) {
                self.likes_count = Some(self.likes_count.unwrap_or(0) + 1);
            }
        }

        impl From<GameTeam> for GameTeamView {
            fn from(team: GameTeam) -> Self {
                GameTeamView {
                    id: team.id,
                    session_id: team.session_id,
                    name: team.name,
                    color: team.color,
                    score: team.score,
                    players: Vec::new(), // this will be populated with a join
                }
            }
        }

        impl From<CreateTeamView> for NewGameTeam {
            fn from(view: CreateTeamView) -> Self {
                NewGameTeam {
                    session_id: Uuid::new_v4(), // this will be set when creating
                    name: view.name,
                    color: view.color,
                }
            }
        }

        impl From<TeamPlayer> for TeamPlayerView {
            fn from(player: TeamPlayer) -> Self {
                TeamPlayerView {
                    team_id: player.team_id,
                    user_id: player.user_id,
                    user: None, // this will be populated with a join
                }
            }
        }

        impl From<UserGameStats> for UserGameStatsView {
            fn from(stats: UserGameStats) -> Self {
                UserGameStatsView {
                    user_id: stats.user_id,
                    games_played: stats.games_played,
                    rooms_created: stats.rooms_created,
                    total_drawing_time: stats.total_drawing_time,
                    canvases_saved: stats.canvases_saved,
                    favorite_game_mode: stats.favorite_game_mode,
                }
            }
        }

        impl From<SavedCanvas> for SavedCanvasView {
            fn from(canvas: SavedCanvas) -> Self {
                SavedCanvasView {
                    id: canvas.id,
                    room_id: canvas.room_id,
                    session_id: canvas.session_id,
                    created_by: canvas.created_by,
                    title: canvas.title,
                    description: canvas.description,
                    canvas_data: canvas.canvas_data,
                    thumbnail_url: canvas.thumbnail_url,
                    is_public: canvas.is_public,
                    likes_count: canvas.likes_count,
                    creator: None, // this will be populated with a join
                }
            }
        }

        impl From<CreateCanvasView> for NewSavedCanvas {
            fn from(view: CreateCanvasView) -> Self {
                NewSavedCanvas {
                    room_id: None, // this will be set when creating
                    session_id: None, // this will be set when creating
                    created_by: 0, // this will be set when creating
                    title: view.title,
                    description: view.description,
                    canvas_data: view.canvas_data,
                    is_public: view.is_public,
                }
            }
        }
    }
}
