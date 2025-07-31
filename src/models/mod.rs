pub mod game;
pub mod rooms;
pub mod users;

pub use users::{UserView, CreateUserView, UpdateUserPreferencesView};
pub use rooms::{
    CanvasRoomView, CreateRoomView, JoinRoomView,
    RoomPlayerView, RoomWithPlayersView,
    GameSessionView, CreateSessionView
};
pub use game::{
    GameTeamView, CreateTeamView, TeamPlayerView,
    UserGameStatsView, SavedCanvasView, CreateCanvasView,
    CanvasGalleryView
};

cfg_if::cfg_if! {
    if #[cfg(feature = "ssr")] {
        pub use users::{User, NewUser, UpdateUserPreferences};
        pub use rooms::{
            CanvasRoom, NewCanvasRoom,
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
