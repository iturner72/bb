use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use axum::extract::FromRef;
        use tokio::sync::broadcast;
        use std::sync::{Arc,Mutex};
        use leptos::prelude::LeptosOptions;

        use crate::cancellable_sse::SseState;
        use crate::database::db::DbPool;
        use crate::handlers::CanvasRoomManager;

        #[derive(FromRef, Clone)]
        pub struct AppState {
            pub leptos_options: LeptosOptions,
            pub pool: DbPool,
            pub sse_state: SseState,
            pub drawing_tx: broadcast::Sender<String>,
            pub user_count: Arc<Mutex<usize>>,
            pub canvas_manager: Option<CanvasRoomManager>,

        }

        impl AppState {
            pub fn new(leptos_options: LeptosOptions, pool: DbPool) -> Self {
                let (drawing_tx, _) = broadcast::channel(100);
                Self {
                    leptos_options,
                    pool,
                    sse_state: SseState::new(),
                    drawing_tx,
                    user_count: Arc::new(Mutex::new(0)),
                    canvas_manager: Some(CanvasRoomManager::new()),
                }
            }
        }
    }
}
