use cfg_if::cfg_if;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::canvas_sync::{
    client_state::ClientCanvasState,
    types::{CanvasMessage, Point},
};

cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use web_sys::{js_sys, CanvasRenderingContext2d};
        use wasm_bindgen::closure::Closure;
    }
}

#[component]
pub fn OTDrawingCanvas() -> impl IntoView {
    let (connected, set_connected) = signal(false);
    let (color, set_color) = signal(String::from("#000000"));
    let (brush_size, set_brush_size) = signal(5);
    let (is_drawing, set_is_drawing) = signal(false);
    let (room_id, _set_room_id) = signal(String::from("default-room"));
    let (user_id, _set_user_id) = signal(generate_user_id());
    let (status_message, set_status_message) = signal(String::new());

    // OT-specific state
    let canvas_state = RwSignal::new(ClientCanvasState::new(user_id.get()));
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // WebSocket management
    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            use std::cell::RefCell;
            thread_local! {
                static WEBSOCKET: RefCell<Option<web_sys::WebSocket>> = const { RefCell::new(None) };
            }
        }
    }

    // send message to server
    let send_message = move |message: CanvasMessage| {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                WEBSOCKET.with(|ws_cell| {
                    if let Some(ws) = ws_cell.borrow().as_ref() {
                        if let Ok(json) = serde_json::to_string(&message) {
                            let _ = ws.send_with_str(&json);
                        }
                    }
                });
            }
        }
    };

    // redraw entire canvas from current state
    let redraw_canvas = move || {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                if let Some(canvas) = canvas_ref.get() {
                    let context = canvas.get_context("2d").unwrap().unwrap()
                        .dyn_into::<CanvasRenderingContext2d>().unwrap();

                    // clear canvas
                    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

                    // draw all visible strokes
                    canvas_state.update(|client_state| {
                        let strokes = client_state.get_visible_strokes();

                        for stroke in strokes {
                            if stroke.points.len() > 1 {
                                context.begin_path();
                                context.set_stroke_style_str(&stroke.color);
                                context.set_line_width(stroke.brush_size as f64);
                                context.set_line_cap("round");
                                context.set_line_join("round");

                                // move to first point
                                let first_point = &stroke.points[0];
                                context.move_to(first_point.x, first_point.y);

                                // draw lines to subsequent points
                                for point in stroke.points.iter().skip(1) {
                                    context.line_to(point.x, point.y);
                                }

                                context.stroke();
                            }
                        }
                    });
                }
            }
        }
    };

    let handle_canvas_message = move |message: CanvasMessage| {
        match message {
            CanvasMessage::StateSync(state) => {
                canvas_state.update(|client_state| {
                    client_state.sync_with_server_state(state);
                });
                redraw_canvas();
                set_status_message.set("Synced".to_string());
            }
            CanvasMessage::OperationAck {
                operation_id,
                server_sequence,
            } => {
                canvas_state.update(|client_state| {
                    client_state.handle_server_ack(&operation_id, server_sequence);
                });
            }
            CanvasMessage::RemoteOperation(operation) => {
                canvas_state.update(|client_state| {
                    let transformed_op = client_state.handle_remote_operation(operation);
                    log::info!(
                        "Applied remote operation: {:?}",
                        transformed_op.operation_type
                    )
                });
                redraw_canvas();
            }
            CanvasMessage::CanvasSaved { url } => {
                // trigger download
                cfg_if! {
                    if #[cfg(feature = "hydrate")] {
                        let window = web_sys::window().unwrap();
                        let document = window.document().unwrap();

                        let link = document.create_element("a").unwrap()
                            .dyn_into::<web_sys::HtmlAnchorElement>().unwrap();

                        link.set_href(&url);
                        link.set_download(&format!("canvas-{}.svg", js_sys::Date::now()));

                        document.body().unwrap().append_child(&link).unwrap();
                        link.click();
                        document.body().unwrap().remove_child(&link).unwrap();

                        set_status_message.set("Canvas saved".to_string());
                    }
                }
            }
            CanvasMessage::Error(error) => {
                log::error!("Canvas error: {}", error);
                set_status_message.set(format!("Error: {}", error));
            }
            _ => {}
        }
    };

    // initialize WebSocket with OT message handling
    let setup_websocket = move || {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                let protocol = if web_sys::window().unwrap().location().protocol().unwrap() == "https:" {
                    "wss"
                } else {
                    "ws"
                };

                let ws_url = format!(
                    "{}://{}/ws/canvas/{}",
                    protocol,
                    web_sys::window().unwrap().location().host().unwrap(),
                    room_id.get()
                );

                match web_sys::WebSocket::new(&ws_url) {
                    Ok(ws) => {
                        let open_closure = Closure::wrap(Box::new(move || {
                            set_connected.set(true);
                            set_status_message.set("Connected".to_string());

                            // request initial state
                            send_message(CanvasMessage::RequestState);
                        }) as Box<dyn FnMut()>);

                        let close_closure = Closure::wrap(Box::new(move || {
                            set_connected.set(false);
                            set_status_message.set("Disconnected".to_string());
                        }) as Box<dyn FnMut()>);

                        let message_closure = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                            if let Some(text) = e.data().as_string() {
                                match serde_json::from_str::<CanvasMessage>(&text) {
                                    Ok(message) => handle_canvas_message(message),
                                    Err(e) => {
                                        log::error!("Failed to parse message: {}", e);
                                    }
                                }
                            }
                        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

                        ws.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
                        ws.set_onclose(Some(close_closure.as_ref().unchecked_ref()));
                        ws.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));

                        open_closure.forget();
                        close_closure.forget();
                        message_closure.forget();

                        WEBSOCKET.with(|ws_cell| {
                            *ws_cell.borrow_mut() = Some(ws);
                        });
                    },
                    Err(err) => {
                        log::error!("Failed to connect to WebSocket: {:?}", err);
                        set_status_message.set("Connection failed".to_string());
                    }
                }
            }
        }
    };

    let draw_line_segment = move |x: f64, y: f64| {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                if let Some(canvas) = canvas_ref.get() {
                    let context = canvas.get_context("2d").unwrap().unwrap()
                        .dyn_into::<CanvasRenderingContext2d>().unwrap();

                    canvas_state.update(|client_state| {
                        if let Some(ref current_stroke) = client_state.current_stroke {
                            if let Some(last_point) = current_stroke.points.last() {
                                context.begin_path();
                                context.move_to(last_point.x, last_point.y);
                                context.line_to(x, y);
                                context.set_line_cap("round");
                                context.set_line_width(current_stroke.brush_size as f64);
                                context.set_stroke_style_str(&current_stroke.color);
                                context.stroke();
                            }
                        }
                    });
                }
            }
        }
    };

    // drawing event handlers
    let start_drawing = move |x: f64, y: f64| {
        set_is_drawing.set(true);

        canvas_state.update(|client_state| {
            let point = Point {
                x,
                y,
                pressure: None,
            };
            client_state.start_stroke(color.get(), brush_size.get(), point);
        });
    };

    let continue_drawing = move |x: f64, y: f64| {
        if is_drawing.get() {
            canvas_state.update(|client_state| {
                let point = Point {
                    x,
                    y,
                    pressure: None,
                };
                client_state.add_to_stroke(point);
            });

            // draw line immediately for responsivness
            draw_line_segment(x, y);
        }
    };

    let finish_drawing = move || {
        if is_drawing.get() {
            set_is_drawing.set(false);

            canvas_state.update(|client_state| {
                if let Some(operation) = client_state.finish_stroke() {
                    send_message(CanvasMessage::SubmitOperation(operation));
                }
            });
        }
    };

    // undo last operation
    let undo = move |_: web_sys::MouseEvent| {
        canvas_state.update(|client_state| {
            // get the operation ID first, then drop the reference
            let last_op_id = {
                let undoable_ops = client_state.get_undoable_operations();
                undoable_ops.last().map(|op| op.id.clone())
            };

            // now we can create the undo operation with a mutable borrow
            if let Some(op_id) = last_op_id {
                let undo_operation = client_state.create_undo(op_id);
                send_message(CanvasMessage::SubmitOperation(undo_operation));
            }
        });
    };

    let clear_canvas = move |_: web_sys::MouseEvent| {
        canvas_state.update(|client_state| {
            let clear_operation = client_state.create_clear();
            send_message(CanvasMessage::SubmitOperation(clear_operation));
        });
    };

    let save_canvas = move |_: web_sys::MouseEvent| {
        send_message(CanvasMessage::SaveCanvas);
    };

    // mouse event handlers
    let on_mouse_down = move |e: web_sys::MouseEvent| {
        start_drawing(e.offset_x() as f64, e.offset_y() as f64);
    };

    let on_mouse_move = move |e: web_sys::MouseEvent| {
        continue_drawing(e.offset_x() as f64, e.offset_y() as f64);
    };

    let on_mouse_up = move |_| {
        finish_drawing();
    };

    // touch event handlers
    let on_touch_start = move |e: web_sys::TouchEvent| {
        e.prevent_default();
        let touches = e.touches();
        if touches.length() > 0 {
            cfg_if! {
                if #[cfg(feature = "hydrate")] {
                    if let Some(touch) = js_sys::try_iter(&touches)
                        .unwrap()
                        .unwrap()
                        .next()
                        .and_then(Result::ok)
                    {
                        let touch: web_sys::Touch = touch.dyn_into().unwrap();
                        if let Some(canvas) = canvas_ref.get() {
                            let rect = canvas.get_bounding_client_rect();
                            let x = touch.client_x() as f64 - rect.left();
                            let y = touch.client_y() as f64 - rect.top();
                            start_drawing(x, y);
                        }
                    }
                }
            }
        }
    };

    let on_touch_move = move |e: web_sys::TouchEvent| {
        e.prevent_default();
        if is_drawing.get() {
            let touches = e.touches();
            if touches.length() > 0 {
                cfg_if! {
                    if #[cfg(feature = "hydrate")] {
                        if let Some(touch) = js_sys::try_iter(&touches)
                            .unwrap()
                            .unwrap()
                            .next()
                            .and_then(Result::ok)
                        {
                            let touch: web_sys::Touch = touch.dyn_into().unwrap();
                            if let Some(canvas) = canvas_ref.get() {
                                let rect = canvas.get_bounding_client_rect();
                                let x = touch.client_x() as f64 - rect.left();
                                let y = touch.client_y() as f64 - rect.top();
                                continue_drawing(x, y);
                            }
                        }
                    }
                }
            }
        }
    };

    let on_touch_end = move |e: web_sys::TouchEvent| {
        e.prevent_default();
        finish_drawing();
    };

    // keyboard shortcurs
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.ctrl_key() || ev.meta_key() {
            match ev.key().as_str() {
                "z" => {
                    undo(web_sys::MouseEvent::new("click").unwrap());
                    ev.prevent_default();
                }
                "s" => {
                    save_canvas(web_sys::MouseEvent::new("click").unwrap());
                    ev.prevent_default();
                }
                _ => {}
            }
        }
    };

    // initialize WebSocket and keyboard listener
    let _ = RenderEffect::new(move |_| {
        setup_websocket();

        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                let window = web_sys::window().unwrap();
                let closure = Closure::wrap(Box::new(handle_keydown) as Box<dyn FnMut(_)>);
                window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
                closure.forget();
            }
        }

        || {
            // cleanup
            cfg_if! {
                if #[cfg(feature = "hydrate")] {
                    WEBSOCKET.with(|ws_cell| {
                        if let Some(ws) = ws_cell.borrow_mut().take() {
                            ws.close().unwrap_or_default();
                        }
                    });
                }
            }
        }
    });

    // helper function
    fn generate_user_id() -> String {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                use web_sys::js_sys::Math;
                format!("user-{}", (Math::random() * 10000.0) as u32)
            } else {
                return "server-side-user".to_string();
            }
        }
    }

    view! {
        <div class="flex flex-col items-center space-y-4">
            <h2 class="text-2xl font-bold">"Operational Transform Canvas"</h2>

            // status bar
            <div class="flex items-center space-x-4 text-sm">
                <div class=format!(
                    "px-2 py-1 rounded {}",
                    if connected.get() {
                        "bg-mint-500 text-gray-300"
                    } else {
                        "bg-salmon-400 text-wenge-500"
                    },
                )>{move || if connected.get() { "Connected" } else { "Disconnected" }}</div>
                <div class="text-gray-600">{status_message}</div>
                <div class="text-gray-500">
                    {move || {
                        canvas_state
                            .with(|client_state| {
                                format!(
                                    "Pending: {} | Can undo: {}",
                                    client_state.get_pending_operations().len(),
                                    if client_state.can_undo() { "Yes" } else { "No" },
                                )
                            })
                    }}
                </div>
            </div>

            // controls
            <div class="flex items-center space-x-4 flex-wrap">
                <div class="flex items-center">
                    <label for="color-picker" class="mr-2">
                        "Color:"
                    </label>
                    <input
                        id="color-picker"
                        type="color"
                        value=color
                        on:change=move |e| set_color.set(event_target_value(&e))
                        class="h-8 w-12"
                    />
                </div>

                <div class="flex items-center">
                    <label for="brush-size" class="mr-2">
                        "Brush:"
                    </label>
                    <input
                        id="brush-size"
                        type="range"
                        min="1"
                        max="30"
                        value=brush_size
                        on:input=move |e| {
                            set_brush_size(event_target_value(&e).parse::<u32>().unwrap_or(5));
                        }
                        class="w-24"
                    />
                    <span class="ml-1 w-8">{brush_size}</span>
                </div>

                <div class="flex space-x-2">
                    <button
                        on:click=undo
                        class="bg-teal-500 hover:bg-teal-600 text-gray-200 px-3 py-1 rounded text-sm disabled:bg-gray-400 disabled:cursor-not-allowed"
                        disabled=move || {
                            canvas_state.with(|canvas_state| { !canvas_state.can_undo() })
                        }
                    >
                        "Undo (Ctrl+Z)"
                    </button>

                    <button
                        on:click=save_canvas
                        class="bg-mint-400 hover:bg-mint-600 text-gray-700 px-3 py-1 rounded text-sm"
                    >
                        "Save (Ctrl+S)"
                    </button>

                    <button
                        on:click=clear_canvas
                        class="bg-salmon-500 hover:bg-salmon-600 text-gray-200 px-3 py-1 rounded text-sm"
                    >
                        "Clear"
                    </button>
                </div>
            </div>

            // canvas
            <div class="border-2 border-gray-300 rounded shadow-lg">
                <canvas
                    node_ref=canvas_ref
                    width="800"
                    height="600"
                    on:mousedown=on_mouse_down
                    on:mousemove=on_mouse_move
                    on:mouseup=on_mouse_up
                    on:mouseout=move |_| finish_drawing()
                    on:touchstart=on_touch_start
                    on:touchmove=on_touch_move
                    on:touchend=on_touch_end
                    on:touchcancel=move |e: web_sys::TouchEvent| {
                        e.prevent_default();
                        finish_drawing();
                    }
                    class="bg-gray-100 touch-none cursor-crosshair"
                ></canvas>
            </div>

            <div class="text-cs text-gray-700 text-center max-w-lg">
                "Operational Transform ensures consistency across all connected clients. "
                "Draw collabortively with automatic conflict resolution!"
            </div>
        </div>
    }
}
