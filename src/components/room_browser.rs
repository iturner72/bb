use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_fetch::QueryClient;
use std::sync::Arc;
use uuid::Uuid;

use super::drawing_rooms::*;
use crate::auth::context::AuthContext;
use crate::models::{CanvasRoomView, JoinRoomView};
use crate::components::drawing_rooms::CreateDrawingRoom;

async fn fetch_rooms_list() -> Result<Vec<RoomListItem>, ServerFnError> {
    list_rooms().await
}

#[component]
pub fn RoomBrowser() -> impl IntoView {
    let (show_create_form, set_show_create_form) = signal(false);
    let (join_code, set_join_code) = signal(String::new());
    let (_current_room, set_current_room) = signal(None::<Uuid>);

    let navigate = use_navigate();
    let navigate_arc = Arc::new(navigate);

    let client: QueryClient = expect_context();

    // get auth context
    let auth = use_context::<AuthContext>()
        .expect("AuthContext should be available");

    // reactive key dependent on auth state
    let rooms_query_key = move || {
        // only return Some when auth is not loading, to prevent query when not ready
        if !auth.is_loading.get() && auth.is_authenticated.get() {
            Some(()) // key for rooms query
        } else {
            None
        }
    };

    let rooms_resource = client.resource(fetch_rooms_list, rooms_query_key);

    let join_room_action = ServerAction::<JoinRoom>::new();

    // clone the Arc for the Effect
    {
        let navigate_for_effect = navigate_arc.clone();
        Effect::new(move |_| {
            if let Some(Ok(room_details)) = join_room_action.value().get() {
                let room_id = room_details.room.id;
                set_current_room(Some(room_id));
                client.invalidate_query(fetch_rooms_list, &());
                navigate_for_effect(&format!("/room/{}", room_id), Default::default());
            } else if let Some(Err(e)) = join_room_action.value().get() {
                log::error!("Failed to join room: {}", e);
            }
        });
    }

    let refresh_rooms = move || {
        client.invalidate_query(fetch_rooms_list, &());
    };


    // clone Arc for use in view closures
    let navigate_for_create = navigate_arc.clone();

    view! {
        <div class="max-w-6xl mx-auto p-6 space-y-6">
            <div class="flex justify-between items-center">
                <h1 class="text-3xl font-bold text-gray-800 dark:text-gray-200">"Drawing Rooms"</h1>
                <div class="flex items-center space-x-4">
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

                <ActionForm action=join_room_action>
                    <div class="flex space-x-3">
                        <input
                            type="text"
                            name="join_data[room_code]"
                            placeholder="Enter room code (e.g., DRABCD12)"
                            value=join_code
                            on:input=move |ev| set_join_code(event_target_value(&ev))
                            class="flex-1 px-3 py-2 border border-gray-300 dark:border-teal-600 rounded-md
                            bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                            focus:outline-none focus:ring-2 focus:ring-seafoam-500"
                        />
                        <button
                            type="submit"
                            disabled=move || {
                                join_room_action.pending().get()
                                    || join_code.get().trim().is_empty()
                            }
                            class="px-6 py-2 bg-mint-600 hover:bg-mint-700 text-white rounded-md
                            disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
                        >
                            {move || {
                                if join_room_action.pending().get() { "Joining..." } else { "Join" }
                            }}
                        </button>
                    </div>
                </ActionForm>

                // Add error display for join action
                {move || {
                    join_room_action
                        .value()
                        .get()
                        .and_then(|result| {
                            result
                                .err()
                                .map(|e| {
                                    view! {
                                        <div class="mt-3 p-3 bg-salmon-100 dark:bg-salmon-900 border border-salmon-300 dark:border-salmon-700 rounded-md">
                                            <p class="text-sm text-salmon-700 dark:text-salmon-300">
                                                "Failed to join room: "{e.to_string()}
                                            </p>
                                        </div>
                                    }
                                        .into_any()
                                })
                        })
                }}
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

            // rooms list with leptos-fetch
            <div class="bg-white dark:bg-teal-800 p-4 rounded-lg shadow-md">
                <div class="flex justify-between items-center mb-4">
                    <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">
                        "Public Rooms"
                    </h2>
                    <button
                        on:click=move |_| refresh_rooms()
                        class="px-3 py-1 text-sm bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 
                             text-gray-700 dark:text-gray-200 rounded transition-colors"
                    >
                        "Refresh"
                    </button>
                </div>

                <Suspense fallback=|| {
                    view! {
                        <div class="text-center py-8">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-seafoam-600 mx-auto"></div>
                            <p class="text-gray-500 dark:text-gray-400 mt-2">"Loading rooms..."</p>
                        </div>
                    }
                }>
                    {move || {
                        Suspend::new(async move {
                            match rooms_resource.await {
                                Some(Ok(room_list)) => {
                                    if room_list.is_empty() {
                                        view! {
                                            <div class="text-center py-8">
                                                <p class="text-gray-500 dark:text-gray-400">
                                                    "No public rooms available. Create one to get started!"
                                                </p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                                                <For
                                                    each=move || room_list.clone()
                                                    key=|room_item| room_item.room.id
                                                    children=move |room_item| {
                                                        let room_code = room_item.room.room_code.clone();
                                                        view! {
                                                            <RoomCard 
                                                                room_item=room_item.clone()
                                                                on_join=Callback::new(move |_room_id| {
                                                                    set_join_code(room_code.clone());
                                                                    join_room_action.dispatch(JoinRoom {
                                                                        join_data: JoinRoomView {
                                                                            room_code: room_code.clone()
                                                                        }
                                                                    });
                                                                })
                                                                on_delete=Callback::new(move |_room_id| {
                                                                    refresh_rooms();
                                                                })
                                                            />
                                                        }
                                                    }
                                                />
                                            </div>
                                        }.into_any()
                                    }
                                },
                                Some(Err(e)) => {
                                    view! {
                                        <div class="text-center py-8">
                                            <div class="mb-4">
                                                <svg class="mx-auto h-12 w-12 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" 
                                                          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.5c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z" />
                                                </svg>
                                            </div>
                                            <p class="text-red-600 dark:text-red-400 mb-2">
                                                "Failed to load rooms"
                                            </p>
                                            <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                                {e.to_string()}
                                            </p>
                                            <button
                                                on:click=move |_| refresh_rooms()
                                                class="px-4 py-2 bg-mint-600 hover:bg-mint-700 text-white rounded transition-colors"
                                            >
                                                "Try Again"
                                            </button>
                                        </div>
                                    }.into_any()
                                },
                                None => {
                                    view! {
                                        <div class="text-center py-8">
                                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-seafoam-600 mx-auto"></div>
                                            <p class="text-gray-500 dark:text-gray-400 mt-2">"Loading rooms..."</p>
                                        </div>
                                    }.into_any()
                                }
                            }
                        })
                    }}
                </Suspense>
            </div>
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

    let create_room_action = ServerAction::<CreateDrawingRoom>::new();

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

    view! {
        <div class="bg-white dark:bg-teal-800 p-6 rounded-lg shadow-md">
            <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200 mb-4">
                "Create New Room"
            </h2>

            // Use ActionForm for proper HTTP POST requests
            <ActionForm action=create_room_action>
                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Room Name"
                        </label>
                        <input
                            type="text"
                            name="room_data[name]"
                            value=room_name
                            on:input=move |ev| set_room_name(event_target_value(&ev))
                            placeholder="Enter room name"
                            required
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
                                name="room_data[max_players]"
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
                                name="room_data[game_mode]"
                                on:change=move |ev| set_game_mode(event_target_value(&ev))
                                class="w-full px-3 py-2 border border-gray-300 dark:border-teal-600 rounded-md
                                bg-white dark:bg-teal-700 text-gray-800 dark:text-gray-200
                                focus:outline-none focus:ring-2 focus:ring-seafoam-500"
                            >
                                <option
                                    value="freeplay"
                                    selected=move || game_mode.get() == "freeplay"
                                >
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
                            name="room_data[is_private]"
                            id="private-room"
                            checked=is_private
                            on:change=move |ev| set_is_private(event_target_checked(&ev))
                            class="h-4 w-4 text-seafoam-600 focus:ring-seafoam-500 border-gray-300 rounded"
                        />
                        <label
                            for="private-room"
                            class="ml-2 text-sm text-gray-700 dark:text-gray-300"
                        >
                            "Private room (invite only)"
                        </label>
                    </div>

                    // Hidden field for settings (optional)
                    <input type="hidden" name="room_data[settings]" value="" />

                    <div class="flex justify-end space-x-3">
                        <button
                            type="submit"
                            disabled=move || {
                                create_room_action.pending().get()
                                    || room_name.get().trim().is_empty()
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

                    // Error display
                    {move || {
                        create_room_action
                            .value()
                            .get()
                            .and_then(|result| {
                                result
                                    .err()
                                    .map(|e| {
                                        view! {
                                            <div class="mt-3 p-3 bg-salmon-100 dark:bg-salmon-900 border border-salmon-300 dark:border-salmon-700 rounded-md">
                                                <p class="text-sm text-salmon-700 dark:text-salmon-300">
                                                    "Error: "{e.to_string()}
                                                </p>
                                            </div>
                                        }
                                            .into_any()
                                    })
                            })
                    }}
                </div>
            </ActionForm>
        </div>
    }.into_any()
}

#[component]
pub fn Toast(
    message: ReadSignal<String>,
    visible: ReadSignal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional, into)] position_class: String,
) -> impl IntoView {
    let opacity_class = move || {
        if visible.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };
    // Use provided position class or default to bottom-right
    let position = if position_class.is_empty() {
        "fixed bottom-4 right-4".to_string()
    } else {
        position_class
    };
    view! {
        <div class=move || {
            format!(
                "{} {} text-xs bg-gray-100 dark:bg-teal-800 text-mint-800 dark:text-mint-600 px-4 py-2 rounded shadow-lg transition-opacity duration-100 z-50",
                opacity_class(),
                position,
            )
        }>
            <div class="relative">
                <button
                    on:click=move |_| on_close.run(())
                    class="absolute -top-1 -left-1 text-danger-500 hover:text-danger-600 text-xs leading-none"
                    title="Close"
                >
                    "×"
                </button>
                <div class="pl-3">
                    <span class="text-mint-800 dark:text-mint-600">{message}</span>
                </div>
            </div>
        </div>
    }.into_any()
}

#[component]
fn RoomCard(
    room_item: RoomListItem,
    #[prop(into)] on_join: Callback<uuid::Uuid>,
    #[prop(into)] on_delete: Callback<uuid::Uuid>,
) -> impl IntoView 
{
    let room = room_item.room;
    let can_join = room_item.can_join;

    let auth = use_context::<AuthContext>()
        .expect("AuthContext should be available");

    let current_user = auth.current_user.get();
    let current_user_id = Memo::new(move |_| {
        if auth.is_loading.get() {
            None
        } else {
            auth.current_user.get().map(|user| user.id)
        }
    });
    
    let is_host = Memo::new(move |_| {
        current_user_id.get().is_some() && room.created_by == current_user_id.get()
    });

    let delete_room_action = ServerAction::<DeleteRoom>::new();

    // Toast state for clipboard feedback
    let (toast_visible, set_toast_visible) = signal(false);
    let (toast_message, set_toast_message) = signal(String::new());

    // handle delete action completion
    Effect::new({
        let on_delete = on_delete;
        let room_id = room.id;
        move |_| {
            if let Some(Ok(())) = delete_room_action.value().get() {
                on_delete.run(room_id);
            }
        }
    });

    let handle_delete = move |_: web_sys::MouseEvent| {
        delete_room_action.dispatch(DeleteRoom { room_id: room.id });
    };

    let handle_copy_code = {
        let room_code = room.room_code.clone();
        move |_: web_sys::MouseEvent| {
            let code = room_code.clone();
            let set_toast_visible = set_toast_visible;
            let set_toast_message = set_toast_message;
            
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(window) = web_sys::window() {
                    let navigator = window.navigator();
                    let clipboard = navigator.clipboard();
                    let promise = clipboard.write_text(&code);
                    
                    match wasm_bindgen_futures::JsFuture::from(promise).await {
                        Ok(_) => {
                            set_toast_message.set("Room code copied!".to_string());
                            set_toast_visible.set(true);
                            
                            // auto-hide toast after 2 seconds
                            set_timeout(
                                move || set_toast_visible.set(false),
                                std::time::Duration::from_secs(2)
                            );
                        }
                        Err(_) => {
                            set_toast_message.set("Failed to copy room code".to_string());
                            set_toast_visible.set(true);
                            
                            // auto-hide error toast after 3 seconds
                            set_timeout(
                                move || set_toast_visible.set(false),
                                std::time::Duration::from_secs(3)
                            );
                        }
                    }
                }
            });
        }
    };

    view! {
        <div class="bg-white dark:bg-teal-800 rounded-lg shadow-md p-6 border border-gray-200 dark:border-teal-600 relative">
            <div class="flex justify-between items-start mb-4">
                <div>
                    <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-1">
                        {room.name}
                    </h3>
                    <div class="flex items-center gap-2 relative">
                        <p class="text-sm text-gray-600 dark:text-gray-400">
                            "Room Code: " <span class="font-mono font-bold">{room.room_code.clone()}</span>
                        </p>
                        <div class="relative">
                            <button
                                on:click=handle_copy_code
                                class="p-1 text-xs text-gray-500 hover:text-seafoam-600 dark:text-gray-400 dark:hover:text-aqua-400 transition-colors"
                                title="Copy room code"
                            >
                                "📋"
                            </button>
                            
                            <div class="absolute left-8 top-0 z-50">
                                <Toast
                                    message=toast_message
                                    visible=toast_visible
                                    on_close=move || set_toast_visible.set(false)
                                    position_class="relative".to_string()
                                />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="flex flex-col items-end">
                    {move || if is_host.get() {
                        view! {
                            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-aqua-100 text-aqua-800 dark:bg-aqua-900 dark:text-aqua-200 mb-2">
                                "Host"
                            </span>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                    <span class="text-xs text-gray-500 dark:text-gray-400">
                        {room.player_count} " players"
                        {move || if let Some(max) = room.max_players {
                            format!(" / {max}")
                        } else {
                            String::new()
                        }}
                    </span>
                </div>
            </div>

            {
                let game_mode_text = match &room.game_mode {
                    Some(mode) => format!("Game Mode: {}", mode),
                    None => "Free Draw".to_string()
                };
                view! {
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
                        {game_mode_text}
                    </p>
                }.into_any()
            }

            <div class="flex justify-between items-center">
                <div class="flex space-x-2">
                    {move || if can_join {
                        view! {
                            <button
                                on:click=move |_| on_join.run(room.id)
                                class="px-3 py-1 bg-seafoam-600 hover:bg-seafoam-700 text-white text-sm rounded transition-colors"
                            >
                                "Join Room"
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <span class="px-3 py-1 bg-gray-400 text-white text-sm rounded cursor-not-allowed">
                                "Room Full"
                            </span>
                        }.into_any()
                    }}
                </div>

                {move || if is_host.get() {
                    view! {
                        <button
                            on:click=handle_delete
                            disabled=move || delete_room_action.pending().get()
                            class="px-3 py-1 bg-salmon-600 hover:bg-salmon-700 text-white text-sm rounded transition-colors disabled:bg-gray-400"
                        >
                            {move || if delete_room_action.pending().get() {
                                "Deleting..."
                            } else {
                                "Delete"
                            }}
                        </button>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>
        </div>
    }.into_any()
}
