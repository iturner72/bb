use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;

use crate::{components::{
    canvas::OTDrawingCanvas, canvas_sync::{
        types::CanvasMessage,
        websocket::{CanvasWebSocket, CanvasWebSocketContext},
    }, drawing_rooms::{get_room_details, kick_player, leave_room}, user_avatar::{AvatarSize, UserAvatar}
}, models::RoomPlayerView};
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
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-teal-900 relative">
            // Mobile-optimized header
            <div class="bg-white dark:bg-teal-800 shadow-sm border-b border-gray-200 dark:border-teal-700 p-2 sm:p-4">
                <div class="flex justify-between items-center">
                    <div class="flex items-center space-x-2 sm:space-x-4 flex-1 min-w-0">
                        <button
                            on:click=move |_| {
                                if let Some(id) = room_id.get() {
                                    leave_room_action.dispatch(id);
                                }
                            }
                            class="text-seafoam-600 dark:text-seafoam-400 hover:text-seafoam-700 
                            dark:hover:text-seafoam-300 flex items-center space-x-1 touch-manipulation p-1 sm:p-0"
                        >
                            <svg
                                class="w-4 h-4 sm:w-5 sm:h-5"
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
                            <span class="hidden sm:inline">"Back to Rooms"</span>
                            <span class="sm:hidden text-xs">"Back"</span>
                        </button>

                        {move || {
                            current_room_data
                                .get()
                                .map(|room_data| {
                                    view! {
                                        <div class="flex items-center space-x-1 sm:space-x-2 flex-1 min-w-0">
                                            <h1 class="text-lg sm:text-xl font-semibold text-gray-800 dark:text-gray-200 truncate">
                                                {room_data.room.name}
                                            </h1>
                                            <span class="text-sm text-gray-500 dark:text-gray-400 hidden sm:inline">
                                                "•"
                                            </span>
                                            <span class="text-xs sm:text-sm text-gray-600 dark:text-gray-300 shrink-0">
                                                <span class="sm:hidden">{room_data.players.len()}</span>
                                                <span class="hidden sm:inline">
                                                    {format!("{} players", room_data.players.len())}
                                                </span>
                                            </span>
                                        </div>
                                    }
                                        .into_any()
                                })
                        }}
                    </div>

                    <div class="flex items-center space-x-2 sm:space-x-4 shrink-0">
                        // Connection status indicator - simplified on mobile
                        <div class="flex items-center space-x-1 sm:space-x-2">
                            <div class=move || {
                                format!(
                                    "w-2 h-2 sm:w-3 sm:h-3 rounded-full {}",
                                    if connected.get() {
                                        "bg-mint-500"
                                    } else {
                                        "bg-yellow-500 animate-pulse"
                                    },
                                )
                            }></div>
                            <span class="text-xs sm:text-sm text-gray-600 dark:text-gray-300 hidden sm:inline">
                                {_connection_status}
                            </span>
                        </div>

                        // Players panel toggle with better mobile design
                        <button
                            on:click=move |_| set_show_players_panel(!show_players_panel.get())
                            class="p-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 
                            dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-teal-700 
                            rounded-md transition-colors touch-manipulation relative"
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
                            // Notification badge for mobile
                            {move || {
                                current_room_data
                                    .get()
                                    .map(|room_data| {
                                        if room_data.players.len() > 0 {
                                            view! {
                                                <span class="absolute -top-1 -right-1 bg-mint-500 text-white text-xs rounded-full h-4 w-4 flex items-center justify-center sm:hidden">
                                                    {room_data.players.len()}
                                                </span>
                                            }
                                                .into_any()
                                        } else {
                                            view! {}.into_any()
                                        }
                                    })
                            }}
                        </button>
                    </div>
                </div>
            </div>

            // Main content area with mobile-responsive layout
            <div class="flex-1 flex overflow-hidden relative">
                // Canvas area - full width on mobile, responsive on desktop
                <div class="flex-1 p-1 sm:p-4">
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

                // Players panel - overlay on mobile, sidebar on desktop
                {move || {
                    (show_players_panel.get() && current_room_data.get().is_some())
                        .then(|| {
                            let room_data = current_room_data.get().unwrap();
                            view! {
                                <>
                                    // Mobile overlay backdrop
                                    <div
                                        class="lg:hidden fixed inset-0 bg-black bg-opacity-50 z-40 transition-opacity"
                                        on:click=move |_| set_show_players_panel(false)
                                    ></div>

                                    // Players panel
                                    <div class="fixed lg:relative inset-y-0 right-0 z-50 lg:z-auto
                                    w-80 sm:w-96 lg:w-80 xl:w-96
                                    bg-white dark:bg-teal-800 
                                    border-l border-gray-200 dark:border-teal-700 
                                    shadow-xl lg:shadow-none
                                    transform transition-transform duration-300 ease-in-out
                                    lg:transform-none
                                    flex flex-col">

                                        // Mobile panel header
                                        <div class="lg:hidden flex items-center justify-between p-4 border-b border-gray-200 dark:border-teal-700">
                                            <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">
                                                Players
                                            </h2>
                                            <button
                                                on:click=move |_| set_show_players_panel(false)
                                                class="p-2 text-gray-600 dark:text-gray-300 hover:text-gray-800 
                                                dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-teal-700 
                                                rounded-md transition-colors touch-manipulation"
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
                                                        d="M6 18L18 6M6 6l12 12"
                                                    />
                                                </svg>
                                            </button>
                                        </div>

                                        // Panel content with proper scrolling
                                        <div class="flex-1 p-3 sm:p-4 overflow-y-auto">
                                            <PlayersPanel
                                                room_data=room_data
                                                room_id=room_id
                                                on_leave=Callback::new(handle_leave_room)
                                                pending=leave_room_action.pending()
                                            />
                                        </div>
                                    </div>
                                </>
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

    let kick_player_action = Action::new(move |(player_id, room_id): &(i32, Uuid)| {
        let player_id = *player_id;
        let room_id = *room_id;
        async move { kick_player(player_id, room_id).await }
    });

    let handle_leave_room = move |_| {
        on_leave.run(());
    };

    let current_user_id = 3; // TODO get from auth context
    let is_host = room.created_by == Some(current_user_id);

    view! {
        <div class="space-y-4 sm:space-y-6 h-full overflow-y-auto">
            // Room info with better mobile spacing
            <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 sm:p-4">
                <h2 class="font-semibold text-gray-800 dark:text-gray-200 mb-3 text-base sm:text-lg">
                    "Room Info"
                </h2>
                <div class="space-y-3 text-sm">
                    <div class="flex justify-between items-center">
                        <span class="text-gray-600 dark:text-gray-400">"Players:"</span>
                        <span class="text-gray-800 dark:text-gray-200 font-medium">
                            {room.player_count}" / "{room.max_players.unwrap_or(999)}
                        </span>
                    </div>
                    <div class="flex justify-between items-center">
                        <span class="text-gray-600 dark:text-gray-400">"Mode:"</span>
                        <span class="text-gray-800 dark:text-gray-200 capitalize font-medium">
                            {room
                                .game_mode
                                .unwrap_or_else(|| "freeplay".to_string())
                                .replace("_", " ")}
                        </span>
                    </div>
                    <div class="flex justify-between items-center">
                        <span class="text-gray-600 dark:text-gray-400">"Privacy:"</span>
                        <span class=format!(
                            "px-2 py-1 text-xs rounded-full font-medium {}",
                            if room.is_private.unwrap_or(false) {
                                "bg-orange-100 dark:bg-orange-900 text-orange-800 dark:text-orange-200"
                            } else {
                                "bg-mint-100 dark:bg-mint-900 text-mint-800 dark:text-mint-200"
                            },
                        )>
                            {if room.is_private.unwrap_or(false) { "Private" } else { "Public" }}
                        </span>
                    </div>
                </div>
            </div>

            // Leave Room Button with better mobile sizing
            <div class="px-1">
                <button
                    on:click=handle_leave_room
                    disabled=move || pending.get()
                    class="w-full px-4 py-3 sm:py-2 bg-salmon-600 hover:bg-salmon-700 active:bg-salmon-800 
                    disabled:bg-salmon-400 disabled:cursor-not-allowed text-white font-medium rounded-lg 
                    transition-all duration-200 flex items-center justify-center space-x-2 touch-manipulation
                    shadow-sm hover:shadow-md"
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

            // Players list with kick functionality
            <div class="flex-1">
                <div class="flex items-center justify-between mb-3 px-1">
                    <h2 class="font-semibold text-gray-800 dark:text-gray-200 text-base sm:text-lg">
                        "Players"
                    </h2>
                    <span class="text-xs sm:text-sm text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded-full">
                        {players.len()}
                    </span>
                </div>
                <div class="space-y-2">
                    <For
                        each=move || players.clone()
                        key=|player| player.id
                        children=move |player| {
                            let can_kick = is_host && player.user_id != current_user_id;

                            view! {
                                <div class="flex items-center space-x-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 
                                transition-colors duration-200 border border-transparent hover:border-gray-200 
                                dark:hover:border-gray-600 cursor-pointer group">
                                    <div class="relative">
                                        <UserAvatar
                                            avatar_url=player
                                                .user
                                                .as_ref()
                                                .and_then(|u| u.avatar_url.clone())
                                            display_name=player
                                                .user
                                                .as_ref()
                                                .and_then(|u| u.display_name.clone().or(u.username.clone()))
                                            size=AvatarSize::Medium
                                        />
                                        // Status indicator
                                        <div class=format!(
                                            "absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-white dark:border-gray-800 {}",
                                            if player.is_active.unwrap_or(false) {
                                                "bg-mint-500 animate-pulse"
                                            } else {
                                                "bg-gray-400"
                                            },
                                        )></div>
                                    </div>

                                    <div class="flex-1 min-w-0">
                                        <div class="flex items-center space-x-2">
                                            <p class="text-sm font-medium text-gray-800 dark:text-gray-200 truncate group-hover:text-gray-900 dark:group-hover:text-gray-100 transition-colors">
                                                {player
                                                    .user
                                                    .as_ref()
                                                    .and_then(|u| {
                                                        u.display_name.as_ref().or(u.username.as_ref())
                                                    })
                                                    .cloned()
                                                    .unwrap_or_else(|| format!("Player {}", player.user_id))}
                                            </p>
                                            // Status badge
                                            {if player.is_active.unwrap_or(false) {
                                                view! {
                                                    <span class="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium bg-mint-100 dark:bg-mint-900 text-mint-800 dark:text-mint-200">
                                                        "Online"
                                                    </span>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <span class="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400">
                                                        "Away"
                                                    </span>
                                                }
                                                    .into_any()
                                            }}
                                        </div>
                                        // Role badge
                                        {player
                                            .role
                                            .as_ref()
                                            .map(|role| {
                                                let role_class = match role.as_str() {
                                                    "host" | "admin" => {
                                                        "bg-purple-100 dark:bg-purple-900 text-purple-800 dark:text-purple-200"
                                                    }
                                                    "moderator" => {
                                                        "bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200"
                                                    }
                                                    _ => {
                                                        "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
                                                    }
                                                };
                                                view! {
                                                    <div class="mt-1">
                                                        <span class=format!(
                                                            "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium capitalize {}",
                                                            role_class,
                                                        )>{role.clone()}</span>
                                                    </div>
                                                }
                                                    .into_any()
                                            })}
                                    </div>

                                    // Kick button (only visible to host for other players)
                                    {if can_kick {
                                        let player_id = player.user_id;
                                        view! {
                                            <div class="flex-shrink-0">
                                                <button
                                                    on:click=move |_| {
                                                        if let Some(room_id) = room_id.get() {
                                                            kick_player_action.dispatch((player_id, room_id));
                                                        }
                                                    }
                                                    disabled=move || kick_player_action.pending().get()
                                                    class="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors duration-200 disabled:opacity-50"
                                                    title="Kick Player"
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
                                                            d="M6 18L18 6M6 6l12 12"
                                                        />
                                                    </svg>
                                                </button>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }}
                                </div>
                            }
                                .into_any()
                        }
                    />
                </div>
            </div>

            // Game session info with better styling
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
                            "bg-mint-100 dark:bg-mint-900 text-mint-800 dark:text-mint-200 border-mint-200 dark:border-mint-700"
                        }
                        Some("waiting") => {
                            "bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200 border-yellow-200 dark:border-yellow-700"
                        }
                        _ => {
                            "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200 border-gray-200 dark:border-gray-600"
                        }
                    };
                    let current_round = session.current_round;
                    let max_rounds = session.max_rounds;

                    view! {
                        <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 sm:p-4 border-t-4 border-mint-500">
                            <h2 class="font-semibold text-gray-800 dark:text-gray-200 mb-3 text-base sm:text-lg flex items-center space-x-2">
                                <svg
                                    class="w-4 h-4 text-mint-600"
                                    fill="none"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M13 10V3L4 14h7v7l9-11h-7z"
                                    />
                                </svg>
                                <span>"Current Session"</span>
                            </h2>
                            <div class="space-y-3 text-sm">
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">"Type:"</span>
                                    <span class="text-gray-800 dark:text-gray-200 capitalize font-medium">
                                        {session_type}
                                    </span>
                                </div>
                                <div class="flex justify-between items-center">
                                    <span class="text-gray-600 dark:text-gray-400">"Status:"</span>
                                    <span class=format!(
                                        "px-3 py-1 text-xs rounded-full font-medium border {}",
                                        status_class,
                                    )>{status_text}</span>
                                </div>
                                {current_round
                                    .map(|round| {
                                        max_rounds
                                            .map(|max_rounds| {
                                                view! {
                                                    <div class="flex justify-between items-center">
                                                        <span class="text-gray-600 dark:text-gray-400">
                                                            "Round:"
                                                        </span>
                                                        <div class="flex items-center space-x-2">
                                                            <div class="flex space-x-1">
                                                                {(1..=max_rounds)
                                                                    .map(|i| {
                                                                        view! {
                                                                            <div class=format!(
                                                                                "w-2 h-2 rounded-full {}",
                                                                                if i <= round {
                                                                                    "bg-mint-500"
                                                                                } else {
                                                                                    "bg-gray-300 dark:bg-gray-600"
                                                                                },
                                                                            )></div>
                                                                        }
                                                                    })
                                                                    .collect::<Vec<_>>()}
                                                            </div>
                                                            <span class="text-gray-800 dark:text-gray-200 font-medium text-xs">
                                                                {round}" / "{max_rounds}
                                                            </span>
                                                        </div>
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
