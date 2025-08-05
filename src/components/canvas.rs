use cfg_if::cfg_if;
use leptos::prelude::*;

use super::canvas_sync::{
    client_state::ClientCanvasState,
    types::{CanvasMessage, Point},
    websocket::CanvasWebSocketContext,
};

cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use web_sys::{js_sys, CanvasRenderingContext2d, HtmlCanvasElement};
        use wasm_bindgen::JsCast;
    }
}

#[component]
pub fn OTDrawingCanvas(#[prop(into)] room_id: String) -> impl IntoView {
    // Get WebSocket interface from context
    let canvas_websocket = CanvasWebSocketContext::expect_context();

    // Get message handler registration callback from context
    let register_handler = expect_context::<Callback<Callback<CanvasMessage>>>();

    // Canvas-specific state
    let (color, set_color) = signal(String::from("#000000"));
    let (brush_size, set_brush_size) = signal(5);
    let (is_drawing, set_is_drawing) = signal(false);
    let (user_id, _set_user_id) = signal(generate_user_id());
    let (status_message, set_status_message) = signal(String::new());

    // OT-specific state
    let canvas_state = RwSignal::new(ClientCanvasState::new(user_id.get()));
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

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

    // Canvas message handler
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
                    );
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

    // Register our message handler with the room page
    let handler_callback = Callback::new(handle_canvas_message);
    register_handler.run(handler_callback);

    let draw_line_segment = move |_x: f64, _y: f64| {
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
                                context.line_to(_x, _y);
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

            // draw line immediately for responsiveness
            draw_line_segment(x, y);
        }
    };

    let finish_drawing = move || {
        if is_drawing.get() {
            set_is_drawing.set(false);

            canvas_state.update(|client_state| {
                if let Some(operation) = client_state.finish_stroke() {
                    canvas_websocket.send(CanvasMessage::SubmitOperation(operation));
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

            if let Some(op_id) = last_op_id {
                let undo_operation = client_state.create_undo(op_id);
                canvas_websocket.send(CanvasMessage::SubmitOperation(undo_operation));
            }
        });
    };

    // save canvas as SVG
    let save_canvas = move |_: web_sys::MouseEvent| {
        canvas_websocket.send(CanvasMessage::SaveCanvas);
    };

    // clear entire canvas
    let clear_canvas = move |_: web_sys::MouseEvent| {
        canvas_state.update(|client_state| {
            let clear_operation = client_state.create_clear();
            canvas_websocket.send(CanvasMessage::SubmitOperation(clear_operation));
        });
    };

    // Mouse event handlers
    let on_mouse_down = move |e: web_sys::MouseEvent| {
        if let Some(canvas) = canvas_ref.get() {
            let rect = canvas.get_bounding_client_rect();
            let x = e.client_x() as f64 - rect.left();
            let y = e.client_y() as f64 - rect.top();
            start_drawing(x, y);
        }
    };

    let on_mouse_move = move |e: web_sys::MouseEvent| {
        if let Some(canvas) = canvas_ref.get() {
            let rect = canvas.get_bounding_client_rect();
            let x = e.client_x() as f64 - rect.left();
            let y = e.client_y() as f64 - rect.top();
            continue_drawing(x, y);
        }
    };

    let on_mouse_up = move |_: web_sys::MouseEvent| {
        finish_drawing();
    };

    cfg_if! {
        if #[cfg(feature = "hydrate")] {
            fn get_canvas_coordinates(
                canvas: &HtmlCanvasElement,
                client_x: f64,
                client_y: f64,
            ) -> (f64, f64) {
                let rect = canvas.get_bounding_client_rect();

                // Get the canvas internal size (actual resolution)
                let canvas_width = canvas.width() as f64;
                let canvas_height = canvas.height() as f64;

                // Get the container size (what getBoundingClientRect returns)
                let container_width = rect.width();
                let container_height = rect.height();

                // Calculate the actual rendered canvas size within the container
                // due to object-fit: contain maintaining aspect ratio
                let canvas_aspect = canvas_width / canvas_height;
                let container_aspect = container_width / container_height;

                let (rendered_width, rendered_height, offset_x, offset_y) = if canvas_aspect > container_aspect {
                    // Canvas is wider - letterboxed top/bottom
                    let rendered_width = container_width;
                    let rendered_height = container_width / canvas_aspect;
                    let offset_x = 0.0;
                    let offset_y = (container_height - rendered_height) / 2.0;
                    (rendered_width, rendered_height, offset_x, offset_y)
                } else {
                    // Canvas is taller - letterboxed left/right
                    let rendered_width = container_height * canvas_aspect;
                    let rendered_height = container_height;
                    let offset_x = (container_width - rendered_width) / 2.0;
                    let offset_y = 0.0;
                    (rendered_width, rendered_height, offset_x, offset_y)
                };

                // Calculate the touch position relative to the canvas element
                let x = client_x - rect.left();
                let y = client_y - rect.top();

                // Subtract the letterbox offset to get position within the actual canvas
                let canvas_relative_x = x - offset_x;
                let canvas_relative_y = y - offset_y;

                // Scale the coordinates to match the canvas resolution
                let canvas_x = (canvas_relative_x * canvas_width) / rendered_width;
                let canvas_y = (canvas_relative_y * canvas_height) / rendered_height;

                (canvas_x, canvas_y)
            }
        }
    }

    // Touch event handlers
    let on_touch_start = move |e: web_sys::TouchEvent| {
        e.prevent_default();
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                let touches = e.touches();
                if let Some(touch) = js_sys::try_iter(&touches)
                    .unwrap()
                    .unwrap()
                    .next()
                    .and_then(Result::ok)
                {
                    let touch: web_sys::Touch = touch.dyn_into().unwrap();
                    if let Some(canvas) = canvas_ref.get() {
                        let (x, y) = get_canvas_coordinates(&canvas, touch.client_x() as f64, touch.client_y() as f64);
                        start_drawing(x, y);
                    }
                }
            }
        }
    };

    let on_touch_move = move |e: web_sys::TouchEvent| {
        e.prevent_default();
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                let touches = e.touches();
                if let Some(touch) = js_sys::try_iter(&touches)
                    .unwrap()
                    .unwrap()
                    .next()
                    .and_then(Result::ok)
                {
                    let touch: web_sys::Touch = touch.dyn_into().unwrap();
                    if let Some(canvas) = canvas_ref.get() {
                        let (x, y) = get_canvas_coordinates(&canvas, touch.client_x() as f64, touch.client_y() as f64);
                        continue_drawing(x, y);
                    }
                }
            }
        }
    };

    let on_touch_end = move |e: web_sys::TouchEvent| {
        e.prevent_default();
        finish_drawing();
    };

    // keyboard shortcuts
    let _handle_keydown = move |ev: web_sys::KeyboardEvent| {
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

    // Setup keyboard listener
    let _ = RenderEffect::new(move |_| {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                let window = web_sys::window().unwrap();
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(_handle_keydown) as Box<dyn FnMut(_)>);
                window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
                closure.forget();
            }
        }

        || {
            // cleanup - keyboard listener will be cleaned up automatically
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
        <div class="w-full h-full flex flex-col bg-white dark:bg-gray-800 rounded-lg shadow-lg overflow-hidden">
            // Enhanced status bar with pending operations and undo status
            <div class="bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600 px-2 sm:px-4 py-2">
                <div class="flex items-center justify-between text-xs sm:text-sm">
                    <div class="flex items-center space-x-2 sm:space-x-4">
                        // Connection status
                        <div class=move || {
                            format!(
                                "px-1.5 sm:px-2 py-0.5 sm:py-1 rounded text-xs {}",
                                if canvas_websocket.is_connected() {
                                    "bg-seafoam-500 text-gray-300"
                                } else {
                                    "bg-salmon-500 text-gray-300"
                                },
                            )
                        }>
                            {move || {
                                if canvas_websocket.is_connected() {
                                    "Connected"
                                } else {
                                    "Disconnected"
                                }
                            }}
                        </div>

                        // Status message - hidden on very small screens
                        <div class="hidden sm:block text-gray-600 dark:text-gray-400">
                            {status_message}
                        </div>

                        // Pending operations and undo status - simplified on mobile
                        <div class="text-gray-500 dark:text-gray-400">
                            {move || {
                                canvas_state
                                    .with(|client_state| {
                                        format!(
                                            "P:{} | U:{}",
                                            client_state.get_pending_operations().len(),
                                            if client_state.can_undo() { "Y" } else { "N" },
                                        )
                                    })
                            }}
                        </div>
                    </div>

                    <div class="text-xs text-gray-500 dark:text-gray-400 hidden md:block">
                        "Room: " {room_id} " • User: " {user_id}
                    </div>
                </div>
            </div>

            // Enhanced toolbar with mobile-first responsive design
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between p-2 sm:p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600 gap-3 sm:gap-0">
                <div class="flex items-center justify-center sm:justify-start space-x-3 sm:space-x-4">
                    // Color picker
                    <div class="flex items-center space-x-2">
                        <label
                            for="color-picker"
                            class="text-sm font-medium text-gray-700 dark:text-gray-300"
                        >
                            "Color:"
                        </label>
                        <input
                            id="color-picker"
                            type="color"
                            prop:value=color
                            on:input=move |e| {
                                let value = event_target_value(&e);
                                set_color.set(value);
                            }
                            class="w-10 h-10 sm:w-10 sm:h-8 rounded border border-gray-300 dark:border-gray-600 cursor-pointer"
                        />
                    </div>

                    // Brush size slider with better mobile sizing
                    <div class="flex items-center space-x-2">
                        <label
                            for="brush-size"
                            class="text-sm font-medium text-gray-700 dark:text-gray-300"
                        >
                            "Brush:"
                        </label>
                        <input
                            id="brush-size"
                            type="range"
                            min="1"
                            max="30"
                            prop:value=brush_size
                            on:input=move |e| {
                                let value = event_target_value(&e).parse().unwrap_or(5);
                                set_brush_size.set(value);
                            }
                            class="w-20 sm:w-24"
                        />
                        <span class="text-sm text-gray-600 dark:text-gray-400 w-6 sm:w-8">
                            {brush_size}
                        </span>
                    </div>
                </div>

                // Action buttons with mobile-optimized layout
                <div class="flex items-center justify-center space-x-2">
                    <button
                        on:click=undo
                        class="bg-teal-500 hover:bg-teal-600 active:bg-teal-700 text-white px-3 py-2 sm:py-1 rounded text-sm disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors touch-manipulation"
                        disabled=move || {
                            canvas_state.with(|canvas_state| { !canvas_state.can_undo() })
                        }
                        title="Undo last action (Ctrl+Z)"
                    >
                        <span class="sm:hidden">"Undo"</span>
                        <span class="hidden sm:inline">"Undo (Ctrl+Z)"</span>
                    </button>

                    <button
                        on:click=save_canvas
                        class="bg-mint-400 hover:bg-mint-600 active:bg-mint-700 text-gray-700 px-3 py-2 sm:py-1 rounded text-sm transition-colors touch-manipulation"
                        title="Save canvas as SVG (Ctrl+S)"
                    >
                        <span class="sm:hidden">"Save"</span>
                        <span class="hidden sm:inline">"Save (Ctrl+S)"</span>
                    </button>

                    <button
                        on:click=clear_canvas
                        class="bg-salmon-500 hover:bg-salmon-600 active:bg-salmon-700 text-white px-3 py-2 sm:py-1 rounded text-sm transition-colors touch-manipulation"
                        title="Clear entire canvas"
                    >
                        "Clear"
                    </button>
                </div>
            </div>

            // Canvas area with mobile-optimized sizing (square on mobile, original aspect ratio on desktop)
            <div class="flex-1 flex items-center justify-center p-1 sm:p-4">
                <div class="w-full aspect-square sm:w-auto sm:h-auto sm:aspect-auto max-w-full max-h-full border-2 border-gray-300 dark:border-gray-600 rounded-lg shadow-lg overflow-hidden">
                    <canvas
                        node_ref=canvas_ref
                        width="800"
                        height="600"
                        class="w-full h-full bg-gray-100 dark:bg-white cursor-crosshair touch-none select-none"
                        style="display: block; object-fit: contain;"
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
                    ></canvas>
                </div>
            </div>

            // Enhanced footer with mobile-friendly text
            <div class="p-2 sm:p-3 bg-gray-50 dark:bg-gray-700 border-t border-gray-200 dark:border-gray-600">
                <div class="text-center text-xs text-gray-600 dark:text-gray-400 max-w-lg mx-auto">
                    <span class="sm:hidden">"Collaborative drawing with real-time sync!"</span>
                    <span class="hidden sm:inline">
                        "Operational Transform ensures consistency across all connected clients. Draw collaboratively with automatic conflict resolution!"
                    </span>
                </div>
            </div>
        </div>
    }
}
