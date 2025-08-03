use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;

use crate::components::{
    canvas::OTDrawingCanvas,
    drawing_rooms::{get_room_details, leave_room},
};
use crate::models::RoomWithPlayersView;

#[component]
pub fn DrawingRoomPage(
    #[prop(into)] room_id: Signal<Option<Uuid>>,
) -> impl IntoView {
    let (current_room_data, set_current_room_data) = signal(None::<RoomWithPlayersView>);
    let (show_players_panel, set_show_players_panel) = signal(true);
    let (connection_status, set_connection_status) = signal("Connecting...".to_string());

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

    // Update current room data when resource changes
    Effect::new(move |_| {
        if let Some(Some(room_data)) = room_details.get() {
            set_current_room_data(Some(room_data));
            set_connection_status("Connected".to_string());
        }
    });

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-teal-900">
            // Header with room info
            <div class="bg-white dark:bg-teal-800 shadow-sm border-b border-gray-200 dark:border-teal-700 p-4">
                <div class="flex justify-between items-center">
                    <div class="flex items-center space-x-4">
                        <a
                            href="/rooms"
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
                        </a>

                        {move || {
                            current_room_data
                                .get()
                                .map(|room_data| {
                                    view! {
                                        <div>
                                            <h1 class="text-xl font-semibold text-gray-800 dark:text-gray-200">
                                                {room_data.room.name.clone()}
                                            </h1>
                                            <p class="text-sm text-gray-500 dark:text-gray-400">
                                                "Room Code: "{room_data.room.room_code.clone()}
                                            </p>
                                        </div>
                                    }
                                        .into_any()
                                })
                        }}
                    </div>

                    <div class="flex items-center space-x-4">
                        // Connection status
                        <div class="flex items-center space-x-2">
                            <div class=format!(
                                "w-2 h-2 rounded-full {}",
                                if connection_status.get() == "Connected" {
                                    "bg-mint-500"
                                } else {
                                    "bg-yellow-500 animate-pulse"
                                },
                            )></div>
                            <span class="text-sm text-gray-600 dark:text-gray-300">
                                {connection_status}
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
                            .map(|_| {
                                view! {
                                    <div class="h-full flex items-center justify-center">
                                        <OTDrawingCanvas />
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
                                    <PlayersPanel room_data=room_data room_id=room_id />
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
) -> impl IntoView {
    let players = room_data.players;
    let room = room_data.room;
    let navigate = use_navigate();

    let leave_room_action = Action::new(move |room_id: &Uuid| {
        let id = *room_id;
        async move { leave_room(id).await }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = leave_room_action.value().get() {
            navigate("/rooms", Default::default());
        }
    });

    let handle_leave_room = move |_| {
        if let Some(id) = room_id.get() {
            leave_room_action.dispatch(id);
        }
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
                    disabled=move || leave_room_action.pending().get()
                    class="w-full px-4 py-2 bg-red-600 hover:bg-red-700 disabled:bg-red-400 
                    disabled:cursor-not-allowed text-white font-medium rounded-md 
                    transition-colors flex items-center justify-center space-x-2"
                >
                    {move || {
                        if leave_room_action.pending().get() {
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

                // Show error if leave room fails
                {move || {
                    leave_room_action
                        .value()
                        .get()
                        .and_then(|result| {
                            if let Err(error) = result {
                                Some(
                                    view! {
                                        <div class="mt-2 p-2 bg-red-100 dark:bg-red-900 border border-red-300 dark:border-red-700 rounded-md">
                                            <p class="text-sm text-red-700 dark:text-red-300">
                                                "Failed to leave room: "{error.to_string()}
                                            </p>
                                        </div>
                                    }
                                        .into_any(),
                                )
                            } else {
                                None
                            }
                        })
                }}
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
                    let status_class = match session.status.as_ref().map(|s| s.as_str()) {
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
