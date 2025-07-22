use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::collections::HashMap;
        use super::types::*;
        use leptos::prelude::*;

        #[derive(Debug, Clone)]
        pub struct ServerCanvasState {
            pub canvas_state: CanvasState,
            pub client_states: HashMap<String, u64>, // client_id -> last_sequence
            pub room_id: String,
        }

        impl ServerCanvasState {
            pub fn new(room_id: String) -> Self {
                Self {
                    canvas_state: CanvasState {
                        strokes: HashMap::new(),
                        operation_history: Vec::new(),
                        server_sequence: 0,
                    },
                    client_states: HashMap::new(),
                    room_id,
                }
            }

            // process incoming operation and return transformed operation + clients to broadcast to
            pub fn process_operation(&mut self, mut operation: Operation) -> (Operation, Vec<String>) {
                // transform against all operations that happened after the client's last known state
                for historical_op in self.canvas_state.operation_history.iter() {
                    if historical_op.server_sequence > operation.server_sequence {
                        operation = transform_operation(operation, historical_op, TransformSide::Right);
                    }
                }

                // assign server sequence number
                self.canvas_state.server_sequence += 1;
                operation.server_sequence = self.canvas_state.server_sequence;

                // apply to server state
                self.apply_operation(&operation);

                // update client's last sequence
                self.client_states.insert(operation.client_id.clone(), operation.sequence);

                // return operation and list of clients to broadcast to (excluding sender)
                let broadcast_clients: Vec<String> = self.client_states
                    .keys()
                    .filter(|&client_id| client_id != &operation.client_id)
                    .cloned()
                    .collect();

                (operation, broadcast_clients)
            }

            fn apply_operation(&mut self, operation: &Operation) {
                match &operation.operation_type {
                    OperationType::DrawStroke { stroke_id, points, color, brush_size } => {
                        let stroke = Stroke {
                            id: stroke_id.clone(),
                            points: points.clone(),
                            color: color.clone(),
                            brush_size: *brush_size,
                            created_by: operation.client_id.clone(),
                            created_at: operation.timestamp,
                            deleted: false,
                        };
                        self.canvas_state.strokes.insert(stroke_id.clone(), stroke);
                    }
                    OperationType::DeleteStroke { stroke_id } => {
                        if let Some(stroke) = self.canvas_state.strokes.get_mut(stroke_id) {
                            stroke.deleted = true;
                        }
                    }
                    OperationType::Clear => {
                        for stroke in self.canvas_state.strokes.values_mut() {
                            stroke.deleted = true;
                        }
                    }
                    OperationType::Undo { target_operation_id } => {
                        if let Some(target_op) = self.canvas_state.operation_history
                            .iter()
                            .find(|op| op.id == *target_operation_id) {

                            match &target_op.operation_type {
                                OperationType::DrawStroke { stroke_id, .. } => {
                                    if let Some(stroke) = self.canvas_state.strokes.get_mut(stroke_id) {
                                        stroke.deleted = true;
                                    }
                                }
                                OperationType::DeleteStroke { stroke_id } => {
                                    if let Some(stroke) = self.canvas_state.strokes.get_mut(stroke_id) {
                                        stroke.deleted = false;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                self.canvas_state.operation_history.push(operation.clone());

                // keep history manageable
                if self.canvas_state.operation_history.len() > 1000 {
                    self.canvas_state.operation_history.drain(0..100);
                }
            }

            pub fn add_client(&mut self, client_id: String) {
                self.client_states.insert(client_id, 0);
            }

            pub fn remove_client(&mut self, client_id: &str) {
                self.client_states.remove(client_id);
            }

            pub fn get_state_for_client(&self) -> CanvasState {
                self.canvas_state.clone()
            }

            pub fn export_canvas_data(&self) -> String {
                // create SVG representation of visible strokes
                let visible_strokes: Vec<&Stroke> = self.canvas_state.strokes
                    .values()
                    .filter(|stroke| !stroke.deleted)
                    .collect();

                let mut svg = String::from(r#"<svg width="800" height="600" xmlns="http://www.w3.org/2000/svg">"#);

                for stroke in visible_strokes {
                    if !stroke.points.is_empty() {
                        let path_data = stroke.points
                            .windows(2)
                            .map(|pair| format!("M{},{} L{},{}", pair[0].x, pair[0].y, pair[1].x, pair[1].y))
                            .collect::<Vec<String>>()
                            .join(" ");

                        svg.push_str(&format!(
                            r#"<path d="{} stroke="{} stroke-width="{}" fill="none" stroke-linecap="round"/>"#,
                            path_data, stroke.color, stroke.brush_size
                        ));
                    }
                }

                svg.push_str("</svg>");
                svg
            }
        }

        #[server(SaveCanvasToFile, "/api")]
        pub async fn save_canvas_to_file(room_id: String) -> Result<String, ServerFnError> {
            use crate::state::AppState;

            let app_state = use_context::<AppState>()
                .ok_or_else(|| ServerFnError::new("AppState not found"))?;

            let canvas_manager = app_state.canvas_manager.as_ref()
                .ok_or_else(|| ServerFnError::new("Canvas manager not found"))?;

            match canvas_manager.save_canvas(room_id).await {
                Ok(url) => Ok(url),
                Err(e) => Err(ServerFnError::new(format!("Failed to save canvas: {}", e)))
            }
        }

        #[server(LoadCanvasState, "/api")]
        pub async fn load_canvas_state(room_id: String) -> Result<CanvasState, ServerFnError> {
            use crate::state::AppState;

            let app_state = use_context::<AppState>()
                .ok_or_else(|| ServerFnError::new("AppState not found"))?;

            let canvas_manager = app_state.canvas_manager.as_ref()
                .ok_or_else(|| ServerFnError::new("Canvas manager not found"))?;

            match canvas_manager.get_room_state(room_id).await {
                Some(state) => Ok(state),
                None => Err(ServerFnError::new("Room not found"))
            }
        }
    }
}

cfg_if! {
    if #[cfg(not(feature = "ssr"))] {
        pub struct ServerCanvasState;

        impl ServerCanvasState {
            pub fn new(_room_id: String) -> Self {
                Self
            }
        }
    }
}
