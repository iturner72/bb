use leptos::prelude::*;
use server_fn::codec::{GetUrl, PostUrl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{
    CanvasRoomView, CreateRoomView, JoinRoomView, RoomWithPlayersView
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomListItem {
    pub room: CanvasRoomView,
    pub can_join: bool,
}

#[server(
    name = CreateDrawingRoom,
    prefix = "/api",
    endpoint = "create_drawing_room",
    input = PostUrl
)]
pub async fn create_drawing_room(room_data: CreateRoomView) -> Result<CanvasRoomView, ServerFnError> {
    use diesel_async::RunQueryDsl;
    use std::fmt;

    use crate::state::AppState;
    use crate::schema::{canvas_rooms, room_players};
    use crate::models::{CanvasRoom, NewCanvasRoom, NewRoomPlayer};

    #[derive(Debug)]
    enum RoomError {
        Pool(String),
        Database(diesel::result::Error),
        #[allow(dead_code)]
        Unauthorized,
    }

    impl fmt::Display for RoomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                RoomError::Pool(e) => write!(f, "Pool error: {e}"),
                RoomError::Database(e) => write!(f, "Database error: {e}"),
                RoomError::Unauthorized => write!(f, "Unauthorized"),
            }
        }
    }

    impl From<RoomError> for ServerFnError {
        fn from(error: RoomError) -> Self {
            ServerFnError::ServerError(error.to_string())
        }
    }

    // TODO: get from auth context
    let user_id = 3;

    let app_state = use_context::<AppState>()
        .expect("Failed to get AppState from context");

    let mut conn = app_state.pool
        .get()
        .await
        .map_err(|e| RoomError::Pool(e.to_string()))?;


    let mut new_room: NewCanvasRoom = room_data.into();
    new_room.created_by = user_id;

    let created_room: CanvasRoom = diesel::insert_into(canvas_rooms::table) 
        .values(&new_room)
        .returning(canvas_rooms::all_columns)
        .get_result(&mut conn)
        .await
        .map_err(RoomError::Database)?;

    // add creator as first player w/ host role
    let new_player = NewRoomPlayer {
        room_id: created_room.id,
        user_id,
        role: Some("host".to_string()),
    };

    diesel::insert_into(room_players::table)
        .values(&new_player)
        .execute(&mut conn)
        .await
        .map_err(RoomError::Database)?;

    let mut room_view: CanvasRoomView = created_room.into();
    room_view.player_count = 1;

    Ok(room_view)
}

#[server(
    prefix = "/api",
    endpoint = "get_public_rooms",
    input = PostUrl, 
)]
pub async fn get_public_rooms() -> Result<Vec<RoomListItem>, ServerFnError> {
    use std::fmt;

    use crate::state::AppState;
    use crate::models::CanvasRoom;

    #[derive(Debug)]
    enum RoomError {
        Pool(String),
        Database(diesel::result::Error),
    }

    impl fmt::Display for RoomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                RoomError::Pool(e) => write!(f, "Pool error: {e}"),
                RoomError::Database(e) => write!(f, "Database error: {e}"),
            }
        }
    }

    impl From<RoomError> for ServerFnError {
        fn from(error: RoomError) -> Self {
            ServerFnError::ServerError(error.to_string())
        }
    }

    let app_state = use_context::<AppState>()
        .expect("Failed to get AppState from context");

    let mut conn = app_state.pool
        .get()
        .await
        .map_err(|e| RoomError::Pool(e.to_string()))?;

    let room_views = CanvasRoom::list_public_with_player_counts(&mut conn)
        .await
        .map_err(RoomError::Database)?;

    let room_list: Vec<RoomListItem> = room_views
        .into_iter()
        .map(|room_view| {
            let can_join = if let Some(max_players) = room_view.max_players {
                (room_view.player_count as i32) < max_players
            } else {
                true
            };
            RoomListItem {
                room: room_view,
                can_join,
            }
        })
        .collect();

    Ok(room_list)
}

#[server(
    name = JoinRoom,
    prefix = "/api",
    endpoint = "join_room",
    input = PostUrl,
)]
pub async fn join_room(join_data: JoinRoomView) -> Result<RoomWithPlayersView, ServerFnError> {
    use std::fmt;
    use crate::state::AppState;
    use crate::models::CanvasRoom;

    #[derive(Debug)]
    enum JoinError {
        Pool(String),
        Database(diesel::result::Error),
        RoomNotFound,
        RoomFull,
        AlreadyInRoom,
        #[allow(dead_code)]
        Unauthorized,
    }

    impl fmt::Display for JoinError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                JoinError::Pool(e) => write!(f, "Pool error: {e}"),
                JoinError::Database(e) => write!(f, "Database error: {e}"),
                JoinError::RoomNotFound => write!(f, "Room not found"),
                JoinError::RoomFull => write!(f, "Room is full"),
                JoinError::AlreadyInRoom => write!(f, "Already in room"),
                JoinError::Unauthorized => write!(f, "Unauthorized"),
            }
        }
    }

    impl From<JoinError> for ServerFnError {
        fn from(error: JoinError) -> Self {
            ServerFnError::ServerError(error.to_string())
        }
    }

    let user_id = 3; // TODO: get from auth context

    let app_state = use_context::<AppState>()
        .expect("Failed to get AppState from context");

    let mut conn = app_state.pool
        .get()
        .await
        .map_err(|e| JoinError::Pool(e.to_string()))?;

    let room = CanvasRoom::find_by_code(&join_data.room_code, &mut conn)
        .await
        .map_err(JoinError::Database)?
        .ok_or(JoinError::RoomNotFound)?;


    // check if user already in room
    if room.has_active_player(user_id, &mut conn).await.map_err(JoinError::Database)? {
        return Err(JoinError::AlreadyInRoom.into());
    }

    // check room capacity
    let current_player_count = room.active_player_count(&mut conn).await.map_err(JoinError::Database)?;

    if let Some(max_players) = room.max_players {
        if current_player_count >= max_players as i64 {
            return Err(JoinError::RoomFull.into());
        }
    }

    // add player to room
    room.add_player(user_id, Some("player".to_string()), &mut conn)
        .await
        .map_err(JoinError::Database)?;

    // get room with all details
    let room_with_players = room.get_with_details(&mut conn)
        .await
        .map_err(JoinError::Database)?;

    Ok(room_with_players)
}

#[server(
    prefix = "/api",
    endpoint = "get_room_details",
    input = GetUrl
)]
pub async fn get_room_details(room_id: Uuid) -> Result<RoomWithPlayersView, ServerFnError> {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use std::fmt;
    use crate::state::AppState;
    use crate::schema::canvas_rooms;
    use crate::models::CanvasRoom;

    #[derive(Debug)]
    enum RoomError {
        Pool(String),
        Database(diesel::result::Error),
        RoomNotFound,
    }

    impl fmt::Display for RoomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                RoomError::Pool(e) => write!(f, "Pool error: {e}"),
                RoomError::Database(e) => write!(f, "Database error: {e}"),
                RoomError::RoomNotFound => write!(f, "Room not found"),
            }
        }
    }

    impl From<RoomError> for ServerFnError {
        fn from(error: RoomError) -> Self {
            ServerFnError::ServerError(error.to_string())
        }
    }

    let app_state = use_context::<AppState>()
        .expect("Failed to get AppState from context");

    let mut conn = app_state.pool
        .get()
        .await
        .map_err(|e| RoomError::Pool(e.to_string()))?;

    let room: CanvasRoom = canvas_rooms::table
        .find(room_id)
        .first(&mut conn)
        .await
        .map_err(|_| RoomError::RoomNotFound)?;

    let room_with_players = room.get_with_details(&mut conn)
        .await
        .map_err(RoomError::Database)?;

    Ok(room_with_players)
}

#[server(
    prefix = "/api",
    endpoint = "leave_room",
    input = PostUrl
)]
pub async fn leave_room(room_id: Uuid) -> Result<(), ServerFnError> {
    use std::fmt;
    use crate::state::AppState;
    use crate::models::RoomPlayer;

    #[derive(Debug)]
    enum LeaveError {
        Pool(String),
        Database(diesel::result::Error),
        #[allow(dead_code)]
        Unauthorized,
    }

    impl fmt::Display for LeaveError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                LeaveError::Pool(e) => write!(f, "Pool error: {e}"),
                LeaveError::Database(e) => write!(f, "Database error: {e}"),
                LeaveError::Unauthorized => write!(f, "Unauthorized"),
            }
        }
    }

    impl From<LeaveError> for ServerFnError {
        fn from(error: LeaveError) -> Self {
            ServerFnError::ServerError(error.to_string())
        }
    }

    let user_id = 3; // TODO: Get from auth context

    let app_state = use_context::<AppState>()
        .expect("Failed to get AppState from context");

    let mut conn = app_state.pool
        .get()
        .await
        .map_err(|e| LeaveError::Pool(e.to_string()))?;

    RoomPlayer::leave_room(user_id, room_id, &mut conn)
        .await
        .map_err(LeaveError::Database)?;

    Ok(())
}

#[server(
    name = DeleteRoom,
    prefix = "/api",
    endpoint = "delete_room",
    input = PostUrl
)]
pub async fn delete_room(room_id: Uuid) -> Result<(), ServerFnError> {
    use std::fmt;
    use crate::state::AppState;
    use crate::models::{CanvasRoom, RoomDeleteError};

    #[derive(Debug)]
    enum DeleteError {
        Pool(String),
        Room(RoomDeleteError),
    }

    impl fmt::Display for DeleteError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DeleteError::Pool(e) => write!(f, "Pool error: {e}"),
                DeleteError::Room(e) => write!(f, "{e}"),
            }
        }
    }

    impl From<DeleteError> for ServerFnError {
        fn from(error: DeleteError) -> Self {
            ServerFnError::ServerError(error.to_string())
        }
    }

    let user_id = 3;

    let app_state = use_context::<AppState>()
        .expect("Failed to get AppState from context");

    let mut conn = app_state.pool
        .get()
        .await
        .map_err(|e| DeleteError::Pool(e.to_string()))?;

    CanvasRoom::delete_room_with_auth_check(room_id, user_id, &mut conn)
        .await
        .map_err(DeleteError::Room)?;

    Ok(())
}
