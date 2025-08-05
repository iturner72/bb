pub mod game;
pub mod rooms;
pub mod users;

pub use game::{
    CanvasGalleryView, CreateCanvasView, CreateTeamView, GameTeamView, SavedCanvasView,
    TeamPlayerView, UserGameStatsView,
};
pub use rooms::{
    CanvasRoomView, CreateRoomView, CreateSessionView, GameSessionView, JoinRoomView,
    RoomPlayerView, RoomWithPlayersView,
};
pub use users::{CreateUserView, UpdateUserPreferencesView, UserView};

cfg_if::cfg_if! {
    if #[cfg(feature = "ssr")] {
        pub use users::{User, NewUser, UpdateUserPreferences};
        pub use rooms::{
            CanvasRoom, NewCanvasRoom, RoomDeleteError,
            RoomPlayer, NewRoomPlayer,
            GameSession, NewGameSession
        };
        pub use game::{
            GameTeam, NewGameTeam,
            TeamPlayer, NewTeamPlayer,
            UserGameStats, NewUserGameStats, UpdateUserGameStats,
            SavedCanvas, NewSavedCanvas
        };
    }
}
