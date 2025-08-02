use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;
use std::sync::Arc;

use crate::models::{CanvasRoomView, CreateRoomView, JoinRoomView};
use super::drawing_rooms::*;
use crate::components::drawing_rooms::create_drawing_room;

#[component]
pub fn RoomBrowser() -> impl IntoView {
    let (show_create_form, set_show_create_form) = signal(false);
    let (join_code, set_join_code) = signal(String::new());
    let (current_room, set_current_room) = signal(None::<Uuid>);

    let navigate = use_navigate();
    
    // Wrap navigate in Arc to make it thread-safe and shareable
    let navigate_arc = Arc::new(navigate);

    // Room list resource
    let rooms = Resource::new(
        || (),
        |_| async move { get_public_rooms().await }
    );

    let refresh_rooms = move || {
        rooms.refetch();
    };

    // Join room by code action
    let join_room_action = Action::new(move |room_code: &String| {
        let code = room_code.clone();
        async move {
            let join_data = JoinRoomView { room_code: code };
            join_room(join_data).await
        }
    });

    // Clone the Arc for the Effect
    {
        let navigate_for_effect = navigate_arc.clone();
        Effect::new(move |_| {
            if let Some(Ok(room_details)) = join_room_action.value().get() {
                let room_id = room_details.room.id;
                set_current_room(Some(room_id));
                navigate_for_effect(&format!("/room/{}", room_id), Default::default());
            } else if let Some(Err(e)) = join_room_action.value().get() {
                log::error!("Failed to join room: {}", e);
            }
        });
    }

    let quick_join = move |_: web_sys::MouseEvent| {
        let code = join_code.get().trim().to_uppercase();
        if !code.is_empty() {
            join_room_action.dispatch(code);
            set_join_code(String::new());
        }
    };

    // Clone Arc for use in view closures
    let navigate_for_create = navigate_arc.clone();
    let navigate_for_rooms = navigate_arc.clone();

    view! {
        <div class="max-w-6xl mx-auto p-6 space-y-6">
            <div class="flex justify-between items-center">
                <h1 class="text-3xl font-bold text-gray-800 dark:text-gray-200">"Drawing Rooms"</h1>
                <div class="flex items-center space-x-4">
                    <button
                        on:click=move |_: web_sys::MouseEvent| refresh_rooms()
                        class="px-4 py-2 bg-gray-500 hover:bg-gray-600 text-white rounded-md transition-colors"
                    >
                        "Refresh"
                    </button>
                    <button
                        on:click=move |_: web_sys::MouseEvent| set_show_create_form(
                            !show_create_form.get(),
                        )
                        class="px-4 py-2 bg-seafoam-600 hover:bg-seafoam-700 text-white rounded-md transition-colors"
                    >
                        {move || if show_create_form.get() { "Cancel" } else { "Create Room" }}
                    </button>
                </div>
            </div>

            // Quick join section
            <div class="bg-white dark:bg-teal-800 p-4 rounded-lg shadow-md">
                <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200 mb-3">
                    "Quick Join"
                </h2>
                <div class="flex space-x-3">
                    <input
                        type="text"
                        placeholder="Enter room code (e.g., DRABCD12)"
                        value=join_code
                        on:input=move |ev| set_join_code(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                let code = join_code.get().trim().to_uppercase();
                                if !code.is_empty() {
                                    join_room_action.dispatch(code);
                                    set_join_code(String::new());
                                }
                            }
                        }
                        class="flex-1 px-3 py-2 border border-gray-300 dark:border-teal-600 rounded-md
                        bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                        focus:outline-none focus:ring-2 focus:ring-seafoam-500"
                    />
                    <button
                        on:click=quick_join
                        disabled=move || {
                            join_room_action.pending().get() || join_code.get().trim().is_empty()
                        }
                        class="px-6 py-2 bg-mint-600 hover:bg-mint-700 text-white rounded-md
                        disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
                    >
                        {move || {
                            if join_room_action.pending().get() { "Joining..." } else { "Join" }
                        }}
                    </button>
                </div>
            </div>

            // Create room form - now uses Arc, thread-safe
            {move || {
                show_create_form
                    .get()
                    .then(|| {
                        let nav = navigate_for_create.clone();
                        view! {
                            <CreateRoomForm on_created=Callback::new(move |room: CanvasRoomView| {
                                let room_id = room.id;
                                set_current_room(Some(room_id));
                                set_show_create_form(false);
                                refresh_rooms();
                                nav(&format!("/room/{}", room_id), Default::default());
                            }) />
                        }
                            .into_any()
                    })
            }}

            // Rooms list - now uses Arc, thread-safe
            <div class="bg-white dark:bg-teal-800 p-4 rounded-lg shadow-md">
                <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200 mb-4">
                    "Public Rooms"
                </h2>

                <Suspense fallback=|| {
                    view! {
                        <div class="text-center py-8">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-seafoam-600 mx-auto"></div>
                            <p class="text-gray-500 dark:text-gray-400 mt-2">"Loading rooms..."</p>
                        </div>
                    }
                        .into_any()
                }>
                    {move || {
                        let nav = navigate_for_rooms.clone();
                        rooms
                            .get()
                            .map(move |result| {
                                match result {
                                    Ok(room_list) => {
                                        if room_list.is_empty() {
                                            view! {
                                                <div class="text-center py-8 text-gray-500 dark:text-gray-400">
                                                    <p>"No public rooms available."</p>
                                                    <p class="text-sm mt-1">"Create one to get started!"</p>
                                                </div>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                                                    <For
                                                        each=move || room_list.clone()
                                                        key=|room_item| room_item.room.id
                                                        children=move |room_item| {
                                                            let nav_for_card = nav.clone();
                                                            view! {
                                                                <RoomCard
                                                                    room_item=room_item.clone()
                                                                    on_join=Callback::new(move |room_id| {
                                                                        nav_for_card(
                                                                            &format!("/room/{}", room_id),
                                                                            Default::default(),
                                                                        );
                                                                    })
                                                                />
                                                            }
                                                                .into_any()
                                                        }
                                                    />
                                                </div>
                                            }
                                                .into_any()
                                        }
                                    }
                                    Err(e) => {
                                        view! {
                                            <div class="text-center py-8 text-red-600 dark:text-red-400">
                                                <p>"Failed to load rooms: "{e.to_string()}</p>
                                                <button
                                                    on:click=move |_: web_sys::MouseEvent| refresh_rooms()
                                                    class="mt-2 px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-md"
                                                >
                                                    "Retry"
                                                </button>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                }
                            })
                    }}
                </Suspense>
            </div>

            // Current room indicator
            {move || {
                current_room
                    .get()
                    .map(|room_id| {
                        view! {
                            <div class="fixed bottom-4 right-4 bg-seafoam-600 text-white p-3 rounded-lg shadow-lg">
                                <p class="text-sm font-medium">"Connected to room"</p>
                                <p class="text-xs opacity-90">{room_id.to_string()}</p>
                            </div>
                        }
                    })
            }}
        </div>
    }.into_any()
}

#[component]
fn CreateRoomForm(
    #[prop(into)] on_created: Callback<CanvasRoomView>,
) -> impl IntoView 
{
    let (room_name, set_room_name) = signal(String::new());
    let (max_players, set_max_players) = signal(8);
    let (is_private, set_is_private) = signal(false);
    let (game_mode, set_game_mode) = signal("freeplay".to_string());

    let create_room_action = Action::new(move |room_data: &CreateRoomView| {
        let data = room_data.clone();
        async move { create_drawing_room(data).await }
    });

    Effect::new(move |_| {
        if let Some(Ok(room)) = create_room_action.value().get() {
            on_created.run(room);
            // Reset form
            set_room_name(String::new());
            set_max_players(8);
            set_is_private(false);
            set_game_mode("freeplay".to_string());
        }
    });

    let submit_form = move |_| {
        let name = room_name.get().trim().to_string();
        if !name.is_empty() {
            let room_data = CreateRoomView {
                name,
                max_players: Some(max_players.get()),
                is_private: Some(is_private.get()),
                game_mode: Some(game_mode.get()),
                settings: None,
            };
            create_room_action.dispatch(room_data);
        }
    };

    view! {
        <div class="bg-white dark:bg-teal-800 p-6 rounded-lg shadow-md">
            <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200 mb-4">
                "Create New Room"
            </h2>

            <div class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Room Name"
                    </label>
                    <input
                        type="text"
                        value=room_name
                        on:input=move |ev| set_room_name(event_target_value(&ev))
                        placeholder="Enter room name"
                        class="w-full px-3 py-2 border border-gray-300 dark:border-teal-600 rounded-md
                        bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                        focus:outline-none focus:ring-2 focus:ring-seafoam-500"
                    />
                </div>

                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Max Players"
                        </label>
                        <select
                            on:change=move |ev| {
                                if let Ok(val) = event_target_value(&ev).parse::<i32>() {
                                    set_max_players(val);
                                }
                            }
                            class="w-full px-3 py-2 border border-gray-300 dark:border-teal-600 rounded-md
                            bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                            focus:outline-none focus:ring-2 focus:ring-seafoam-500"
                        >
                            <option value="4" selected=move || max_players.get() == 4>
                                "4 players"
                            </option>
                            <option value="8" selected=move || max_players.get() == 8>
                                "8 players"
                            </option>
                            <option value="12" selected=move || max_players.get() == 12>
                                "12 players"
                            </option>
                            <option value="16" selected=move || max_players.get() == 16>
                                "16 players"
                            </option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Game Mode"
                        </label>
                        <select
                            on:change=move |ev| set_game_mode(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-teal-600 rounded-md
                            bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                            focus:outline-none focus:ring-2 focus:ring-seafoam-500"
                        >
                            <option value="freeplay" selected=move || game_mode.get() == "freeplay">
                                "Free Play"
                            </option>
                            <option
                                value="guessing_game"
                                selected=move || game_mode.get() == "guessing_game"
                            >
                                "Guessing Game"
                            </option>
                            <option value="teams" selected=move || game_mode.get() == "teams">
                                "Team Battle"
                            </option>
                        </select>
                    </div>
                </div>

                <div class="flex items-center">
                    <input
                        type="checkbox"
                        id="private-room"
                        checked=is_private
                        on:change=move |ev| set_is_private(event_target_checked(&ev))
                        class="h-4 w-4 text-seafoam-600 focus:ring-seafoam-500 border-gray-300 rounded"
                    />
                    <label for="private-room" class="ml-2 text-sm text-gray-700 dark:text-gray-300">
                        "Private room (invite only)"
                    </label>
                </div>

                <div class="flex justify-end space-x-3">
                    <button
                        type="button"
                        on:click=submit_form
                        disabled=move || {
                            create_room_action.pending().get() || room_name.get().trim().is_empty()
                        }
                        class="px-6 py-2 bg-seafoam-600 hover:bg-seafoam-700 text-white rounded-md
                        disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
                    >
                        {move || {
                            if create_room_action.pending().get() {
                                "Creating..."
                            } else {
                                "Create Room"
                            }
                        }}
                    </button>
                </div>

                {move || {
                    create_room_action
                        .value()
                        .get()
                        .and_then(|result| {
                            result
                                .err()
                                .map(|e| {
                                    view! {
                                        <div class="mt-3 p-3 bg-red-100 dark:bg-red-900 border border-red-300 dark:border-red-700 rounded-md">
                                            <p class="text-sm text-red-700 dark:text-red-300">
                                                "Error: "{e.to_string()}
                                            </p>
                                        </div>
                                    }
                                })
                        })
                }}
            </div>
        </div>
    }
}

#[component]
fn RoomCard(
    room_item: RoomListItem,
    #[prop(into)] on_join: Callback<uuid::Uuid>,
) -> impl IntoView 
{
    let room = room_item.room;
    let can_join = room_item.can_join;

    view! {
        <div class="border border-gray-200 dark:border-teal-600 rounded-lg p-4 hover:shadow-md transition-shadow">
            <div class="flex justify-between items-start mb-3">
                <div>
                    <h3 class="font-semibold text-gray-800 dark:text-gray-200 truncate">
                        {room.name.clone()}
                    </h3>
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                        "Code: "{room.room_code.clone()}
                    </p>
                </div>
                <div class=format!(
                    "px-2 py-1 text-xs rounded-full {}",
                    if can_join {
                        "bg-mint-100 dark:bg-mint-900 text-mint-800 dark:text-mint-200"
                    } else {
                        "bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200"
                    },
                )>{if can_join { "Open" } else { "Full" }}</div>
            </div>

            <div class="space-y-2 text-sm text-gray-600 dark:text-gray-400">
                <div class="flex justify-between">
                    <span>"Players:"</span>
                    <span>{room.player_count}" / "{room.max_players.unwrap_or(999)}</span>
                </div>
                <div class="flex justify-between">
                    <span>"Mode:"</span>
                    <span class="capitalize">
                        {room.game_mode.unwrap_or_else(|| "freeplay".to_string()).replace("_", " ")}
                    </span>
                </div>
            </div>

            <button
                on:click=move |_| {
                    if can_join {
                        on_join.run(room.id)
                    }
                }
                disabled=!can_join
                class=format!(
                    "w-full mt-4 px-4 py-2 rounded-md transition-colors {}",
                    if can_join {
                        "bg-seafoam-600 hover:bg-seafoam-700 text-white"
                    } else {
                        "bg-gray-300 dark:bg-gray-600 text-gray-500 dark:text-gray-400 cursor-not-allowed"
                    },
                )
            >
                {if can_join { "Join Room" } else { "Room Full" }}
            </button>
        </div>
    }
}
