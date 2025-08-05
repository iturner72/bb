use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;

use crate::components::{
    canvas::OTDrawingCanvas,
    drawing_rooms::{get_room_details, leave_room},
    canvas_sync::{
        types::CanvasMessage,
        websocket::{CanvasWebSocket, CanvasWebSocketContext},
    },
};
use crate::models::RoomWithPlayersView;

cfg_if::cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use wasm_bindgen::{closure::Closure, JsCast};
    }
}

#[component]
pub fn DrawingRoomPage(
    #[prop(into)] room_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let (current_room_data, set_current_room_data) = signal(None::<RoomWithPlayersView>);
    let (show_players_panel, set_show_players_panel) = signal(true);
    let (_connection_status, _set_connection_status) = signal("Connecting...".to_string());
    let (connected, _set_connected) = signal(false);

    // WebSocket state - managed at room level using thread-local storage
    cfg_if::cfg_if! {
        if #[cfg(feature = "hydrate")] {
            use std::cell::RefCell;
            thread_local! {
                static ROOM_WEBSOCKET: RefCell<Option<web_sys::WebSocket>> = const { RefCell::new(None) };
            }
        }
    }

    // Message handler callback - will be passed to canvas
    let (_message_handler, _set_message_handler) = signal(None::<Callback<CanvasMessage>>);

    // WebSocket send function
    let _send_message = Callback::new(move |_message: CanvasMessage| {
        cfg_if::cfg_if! {
            if #[cfg(feature = "hydrate")] {
                ROOM_WEBSOCKET.with(|ws_cell| {
                    if let Some(ws) = ws_cell.borrow().as_ref() {
                        if let Ok(json) = serde_json::to_string(&_message) {
                            let _ = ws.send_with_str(&json);
                        }
                    }
                });
            }
        }
    });

    // Create WebSocket interface
    let canvas_websocket = CanvasWebSocket::new(
        connected,
        _send_message,
        _connection_status,
    );

    // Setup WebSocket when room_id changes
    let setup_websocket = move |_room_uuid: Uuid| {
        cfg_if::cfg_if! {
            if #[cfg(feature = "hydrate")] {
                // Close existing connection if any
                ROOM_WEBSOCKET.with(|ws_cell| {
                    if let Some(existing_ws) = ws_cell.borrow_mut().take() {
                        let _ = existing_ws.close();
                    }
                });

                let protocol = if web_sys::window().unwrap().location().protocol().unwrap() == "https:" {
                    "wss"
                } else {
                    "ws"
                };

                let ws_url = format!(
                    "{}://{}/ws/canvas/{}/{}",
                    protocol,
                    web_sys::window().unwrap().location().host().unwrap(),
                    _room_uuid,
                    3 // TODO: need actual user_id from auth context
                );

                match web_sys::WebSocket::new(&ws_url) {
                    Ok(ws) => {
                        // Connection opened
                        let set_connected_clone = _set_connected;
                        let set_status_clone = _set_connection_status;
                        let send_message_clone = _send_message;
                        let open_closure = Closure::wrap(Box::new(move || {
                            set_connected_clone.set(true);
                            set_status_clone.set("Connected".to_string());
                            // Request initial state
                            send_message_clone.run(CanvasMessage::RequestState);
                        }) as Box<dyn FnMut()>);

                        // Connection closed
                        let set_connected_clone = _set_connected;
                        let set_status_clone = _set_connection_status;
                        let close_closure = Closure::wrap(Box::new(move || {
                            set_connected_clone.set(false);
                            set_status_clone.set("Disconnected".to_string());
                        }) as Box<dyn FnMut()>);

                        // Message received
                        let message_handler_signal = _message_handler;
                        let message_closure = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                            if let Some(text) = e.data().as_string() {
                                match serde_json::from_str::<CanvasMessage>(&text) {
                                    Ok(message) => {
                                        // Forward to canvas component if handler is set
                                        if let Some(handler) = message_handler_signal.get() {
                                            handler.run(message);
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Failed to parse WebSocket message: {}", e);
                                    }
                                }
                            }
                        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

                        ws.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
                        ws.set_onclose(Some(close_closure.as_ref().unchecked_ref()));
                        ws.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));

                        // Keep closures alive
                        open_closure.forget();
                        close_closure.forget();
                        message_closure.forget();

                        // Store WebSocket in thread-local storage
                        ROOM_WEBSOCKET.with(|ws_cell| {
                            *ws_cell.borrow_mut() = Some(ws);
                        });
                    }
                    Err(err) => {
                        log::error!("Failed to connect to WebSocket: {:?}", err);
                        _set_connection_status.set("Connection failed".to_string());
                    }
                }
            }
        }
    };

    // Load room details when room_id changes
    let room_details = Resource::new(
        room_id,
        |room_id| async move {
            match room_id {
                Some(id) => get_room_details(id).await.ok(),
                None => None,
            }
        },
    );

    // Update current room data and setup WebSocket when resource changes
    Effect::new(move |_| {
        if let Some(Some(room_data)) = room_details.get() {
            set_current_room_data(Some(room_data.clone()));
            
            // Setup WebSocket for this room
            if let Some(room_uuid) = room_id.get() {
                setup_websocket(room_uuid);
            }
        }
    });

    // Cleanup WebSocket on component unmount
    let cleanup_websocket = move || {
        cfg_if::cfg_if! {
            if #[cfg(feature = "hydrate")] {
                ROOM_WEBSOCKET.with(|ws_cell| {
                    if let Some(ws) = ws_cell.borrow_mut().take() {
                        let _ = ws.close();
                    }
                });
            }
        }
    };

    let navigate = use_navigate();

    let leave_room_action = Action::new(move |room_id: &Uuid| {
        let id = *room_id;
        async move { leave_room(id).await }
    });

    // Effect for navigation after successful leave
    Effect::new(move |_| {
        if let Some(Ok(())) = leave_room_action.value().get() {
            navigate("/rooms", Default::default());
        }
    });

    // handle leave room - closes websocket first, then calls server action
    let handle_leave_room = move |_| {
        cleanup_websocket();
        if let Some(id) = room_id.get() {
            leave_room_action.dispatch(id);
        }
    }; 

    // Register message handler from canvas
    let register_message_handler = Callback::new(move |handler: Callback<CanvasMessage>| {
        _set_message_handler.set(Some(handler));
    });

    // Provide WebSocket context to child components
    CanvasWebSocketContext::provide(canvas_websocket);

    // Provide the registration callback as context too
    provide_context(register_message_handler);

    // Cleanup effect
    let _ = RenderEffect::new(move |_| {
        // Effect cleanup function
        move || {
            cleanup_websocket();
        }
    });

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-teal-900">
            // Header with room info
            <div class="bg-white dark:bg-teal-800 shadow-sm border-b border-gray-200 dark:border-teal-700 p-4">
                <div class="flex justify-between items-center">
                    <div class="flex items-center space-x-4">
                        <button
                            on:click=move |_| {
                                if let Some(id) = room_id.get() {
                                    leave_room_action.dispatch(id);
                                }
                            }
                            class="text-seafoam-600 dark:text-seafoam-400 hover:text-seafoam-700 
                            dark:hover:text-seafoam-300 flex items-center space-x-1"
                        >
                            <svg
                                class="w-4 h-4"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M15 19l-7-7 7-7"
                                />
                            </svg>
                            <span>"Back to Rooms"</span>
                        </button>

                        {move || {
                            current_room_data
                                .get()
                                .map(|room_data| {
                                    view! {
                                        <div class="flex items-center space-x-2">
                                            <h1 class="text-xl font-semibold text-gray-800 dark:text-gray-200">
                                                {room_data.room.name}
                                            </h1>
                                            <span class="text-sm text-gray-500 dark:text-gray-400">
                                                "•"
                                            </span>
                                            <span class="text-sm text-gray-600 dark:text-gray-300">
                                                {format!("{} players", room_data.players.len())}
                                            </span>
                                        </div>
                                    }
                                        .into_any()
                                })
                        }}
                    </div>

                    <div class="flex items-center space-x-4">
                        // Connection status indicator
                        <div class="flex items-center space-x-2">
                            <div class=move || {
                                format!(
                                    "w-2 h-2 rounded-full {}",
                                    if connected.get() {
                                        "bg-mint-500"
                                    } else {
                                        "bg-yellow-500 animate-pulse"
                                    },
                                )
                            }></div>
                            <span class="text-sm text-gray-600 dark:text-gray-300">
                                {_connection_status}
                            </span>
                        </div>

                        // Players panel toggle
                        <button
                            on:click=move |_| set_show_players_panel(!show_players_panel.get())
                            class="p-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 
                            dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-teal-700 
                            rounded-md transition-colors"
                            title="Toggle players panel"
                        >
                            <svg
                                class="w-5 h-5"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
                                />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>

            // Main content area
            <div class="flex-1 flex overflow-hidden">
                // Canvas area
                <div class="flex-1 p-4">
                    {move || {
                        room_id
                            .get()
                            .map(|room_uuid| {
                                view! {
                                    <div class="h-full flex items-center justify-center">
                                        <OTDrawingCanvas room_id=room_uuid.to_string() />
                                    </div>
                                }
                                    .into_any()
                            })
                    }}
                </div>

                // Players panel
                {move || {
                    (show_players_panel.get() && current_room_data.get().is_some())
                        .then(|| {
                            let room_data = current_room_data.get().unwrap();
                            view! {
                                <div class="w-80 bg-white dark:bg-teal-800 border-l border-gray-200 dark:border-teal-700 p-4 overflow-y-auto">
                                    <PlayersPanel
                                        room_data=room_data
                                        room_id=room_id
                                        on_leave=Callback::new(handle_leave_room)
                                        pending=leave_room_action.pending()
                                    />
                                </div>
                            }
                                .into_any()
                        })
                }}
            </div>
        </div>
    }.into_any()
}

#[component]
fn PlayersPanel(
    room_data: RoomWithPlayersView,
    #[prop(into)] room_id: Signal<Option<Uuid>>,
    #[prop(into)] on_leave: Callback<()>,
    #[prop(into)] pending: Signal<bool>,
) -> impl IntoView {
    let players = room_data.players;
    let room = room_data.room;

    let handle_leave_room = move |_| {
        on_leave.run(());
    };

    view! {
        <div class="space-y-6">
            // Room info
            <div>
                <h2 class="font-semibold text-gray-800 dark:text-gray-200 mb-3">"Room Info"</h2>
                <div class="space-y-2 text-sm">
                    <div class="flex justify-between">
                        <span class="text-gray-600 dark:text-gray-400">"Players:"</span>
                        <span class="text-gray-800 dark:text-gray-200">
                            {room.player_count}" / "{room.max_players.unwrap_or(999)}
                        </span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 dark:text-gray-400">"Mode:"</span>
                        <span class="text-gray-800 dark:text-gray-200 capitalize">
                            {room
                                .game_mode
                                .unwrap_or_else(|| "freeplay".to_string())
                                .replace("_", " ")}
                        </span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 dark:text-gray-400">"Privacy:"</span>
                        <span class="text-gray-800 dark:text-gray-200">
                            {if room.is_private.unwrap_or(false) { "Private" } else { "Public" }}
                        </span>
                    </div>
                </div>
            </div>

            // Leave Room Button
            <div class="border-t border-gray-200 dark:border-teal-700 pt-4">
                <button
                    on:click=handle_leave_room
                    disabled=move || pending.get()
                    class="w-full px-4 py-2 bg-salmon-600 hover:bg-salmon-700 disabled:bg-salmon-400 
                    disabled:cursor-not-allowed text-white font-medium rounded-md 
                    transition-colors flex items-center justify-center space-x-2"
                >
                    {move || {
                        if pending.get() {
                            view! {
                                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                                <span>"Leaving..."</span>
                            }
                                .into_any()
                        } else {
                            view! {
                                <svg
                                    class="w-4 h-4"
                                    fill="none"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                                    />
                                </svg>
                                <span>"Leave Room"</span>
                            }
                                .into_any()
                        }
                    }}
                </button>
            </div>

            // Players list
            <div>
                <h2 class="font-semibold text-gray-800 dark:text-gray-200 mb-3">
                    "Players ("{players.len()}")"
                </h2>
                <div class="space-y-2">
                    <For
                        each=move || players.clone()
                        key=|player| player.id
                        children=move |player| {
                            view! {
                                <div class="flex items-center space-x-3 p-2 rounded-md hover:bg-gray-50 dark:hover:bg-teal-700">
                                    // Avatar placeholder
                                    <div class="w-8 h-8 bg-seafoam-500 rounded-full flex items-center justify-center text-white text-sm font-medium">
                                        {player
                                            .user
                                            .as_ref()
                                            .and_then(|u| {
                                                u.display_name.as_ref().or(u.username.as_ref())
                                            })
                                            .and_then(|name| name.chars().next())
                                            .unwrap_or('?')}
                                    </div>

                                    <div class="flex-1 min-w-0">
                                        <p class="text-sm font-medium text-gray-800 dark:text-gray-200 truncate">
                                            {player
                                                .user
                                                .as_ref()
                                                .and_then(|u| {
                                                    u.display_name.as_ref().or(u.username.as_ref())
                                                })
                                                .cloned()
                                                .unwrap_or_else(|| format!("Player {}", player.user_id))}
                                        </p>
                                        {player
                                            .role
                                            .as_ref()
                                            .map(|role| {
                                                view! {
                                                    <p class="text-xs text-gray-500 dark:text-gray-400 capitalize">
                                                        {role.clone()}
                                                    </p>
                                                }
                                                    .into_any()
                                            })}
                                    </div>

                                    // Status indicator
                                    <div class=format!(
                                        "w-2 h-2 rounded-full {}",
                                        if player.is_active.unwrap_or(false) {
                                            "bg-mint-500"
                                        } else {
                                            "bg-gray-400"
                                        },
                                    )></div>
                                </div>
                            }
                                .into_any()
                        }
                    />
                </div>
            </div>

            // Game session info (if active)
            {room_data
                .current_session
                .map(|session| {
                    let session_type = session.session_type.replace("_", " ");
                    let status_text = session
                        .status
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string());
                    let status_class = match session.status.as_deref() {
                        Some("active") => {
                            "bg-mint-100 dark:bg-mint-900 text-mint-800 dark:text-mint-200"
                        }
                        Some("waiting") => {
                            "bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200"
                        }
                        _ => "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200",
                    };
                    let current_round = session.current_round;
                    let max_rounds = session.max_rounds;

                    view! {
                        <div>
                            <h2 class="font-semibold text-gray-800 dark:text-gray-200 mb-3">
                                "Current Session"
                            </h2>
                            <div class="space-y-2 text-sm">
                                <div class="flex justify-between">
                                    <span class="text-gray-600 dark:text-gray-400">"Type:"</span>
                                    <span class="text-gray-800 dark:text-gray-200 capitalize">
                                        {session_type}
                                    </span>
                                </div>
                                <div class="flex justify-between">
                                    <span class="text-gray-600 dark:text-gray-400">"Status:"</span>
                                    <span class=format!(
                                        "px-2 py-1 text-xs rounded-full {}",
                                        status_class,
                                    )>{status_text}</span>
                                </div>
                                {current_round
                                    .map(|round| {
                                        max_rounds
                                            .map(|max_rounds| {
                                                view! {
                                                    <div class="flex justify-between">
                                                        <span class="text-gray-600 dark:text-gray-400">
                                                            "Round:"
                                                        </span>
                                                        <span class="text-gray-800 dark:text-gray-200">
                                                            {round}" / "{max_rounds}
                                                        </span>
                                                    </div>
                                                }
                                            })
                                    })}
                            </div>
                        </div>
                    }
                        .into_any()
                })}
        </div>
    }.into_any()
}
