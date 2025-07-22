use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::state::AppState;
use crate::components::canvas_sync::{types::*, server_state::ServerCanvasState,};

#[derive(Clone)]
pub struct CanvasRoomManager {
    pub rooms: Arc<RwLock<HashMap<String, ServerCanvasState>>>,
}

impl CanvasRoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_room(&self, room_id: String) -> ServerCanvasState {
        let mut rooms = self.rooms.write().await;
        if !rooms.contains_key(&room_id) {
            rooms.insert(room_id.clone(), ServerCanvasState::new(room_id.clone()));
        }
        rooms.get(&room_id).unwrap().clone()
    }

    pub async fn process_operation(
        &self,
        room_id: String,
        operation: Operation,
    ) -> Result<(Operation, Vec<String>), String> {
        let mut rooms = self.rooms.write().await;
        if !rooms.contains_key(&room_id) {
            rooms.insert(room_id.clone(), ServerCanvasState::new(room_id.clone()));
        }
        let room = rooms.get_mut(&room_id)
            .ok_or_else(|| "Room not found".to_string())?;
        Ok(room.process_operation(operation))
    }

    pub async fn add_client_to_room(&self, room_id: String, client_id: String) {
        let mut rooms = self.rooms.write().await;
        if !rooms.contains_key(&room_id) {
            rooms.insert(room_id.clone(), ServerCanvasState::new(room_id.clone()));
        }
        if let Some(room) = rooms.get_mut(&room_id) {
            room.add_client(client_id);
        }
    }

    pub async fn remove_client_from_room(&self, room_id: String, client_id: String) {
        let mut rooms = self.rooms.write().await;
        if let Some(room) = rooms.get_mut(&room_id) {
            room.remove_client(&client_id);
        }
    } 

    pub async fn get_room_state(&self, room_id: String) -> Option<CanvasState> {
        let rooms = self.rooms.write().await;
        rooms.get(&room_id).map(|room| room.get_state_for_client())
    }

    pub async fn save_canvas(&self, room_id: String) -> Result<String, String> {
        let rooms = self.rooms.write().await;
        let room = rooms.get(&room_id)
            .ok_or_else(|| "Room not found".to_string())?;

        let svg_data = room.export_canvas_data();

        // return as data URI, save to file storage in real impl
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(svg_data);
        Ok(format!("data:image/svg+xml;base64,{}", encoded))
    }
}

pub async fn canvas_ws_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let client_id = format!("client-{}", uuid::Uuid::new_v4());

    log::info!("Canvas client {} connecting to room {}", client_id, room_id);

    // add to room
    if let Some(canvas_manager) = state.canvas_manager.as_ref() {
        canvas_manager.add_client_to_room(room_id.clone(), client_id.clone()).await;
    }

    ws.on_upgrade(move |socket| handle_canvas_socket(socket, state, room_id, client_id))
}

async fn handle_canvas_socket(
    socket: WebSocket,
    state: AppState,
    room_id: String,
    client_id: String
) {
    let (mut sender, mut receiver) = socket.split();
    let canvas_manager = state.canvas_manager.clone();

    // send initial room state
    if let Some(manager) = &canvas_manager {
        if let Some(canvas_state) = manager.get_room_state(room_id.clone()).await {
            let sync_message = CanvasMessage::StateSync(canvas_state);
            if let Ok(json) = serde_json::to_string(&sync_message) {
                let _ = sender.send(Message::Text(json)).await;
            }
        }
    }

    // task for sending messages to this client
    let mut send_task = {
        let mut rx = state.drawing_tx.subscribe();
        let client_id_clone = client_id.clone();

        tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                // only forawrd messages not from this client
                if let Ok(canvas_msg) = serde_json::from_str::<CanvasMessage>(&message) {
                    let should_forward = match &canvas_msg {
                        CanvasMessage::RemoteOperation(op) => op.client_id != client_id_clone,
                        CanvasMessage::OperationAck { .. } => true, // always send acks 
                        _ => true,
                    };

                    if should_forward {
                        if let Ok(json) = serde_json::to_string(&canvas_msg) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        })
    };

    // task for receiving messages from this client
    let mut recv_task = {
        let canvas_manager = canvas_manager.clone();
        let room_id_clone = room_id.clone();
        let client_id_clone = client_id.clone();
        let tx = state.drawing_tx.clone();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<CanvasMessage>(&text) {
                            Ok(canvas_message) => {
                                let response = handle_canvas_message(
                                    canvas_message,
                                    &canvas_manager,
                                    &room_id_clone,
                                    &client_id_clone,
                                ).await;

                                match response {
                                    Ok(responses) => {
                                        for response_msg in responses {
                                            match response_msg {
                                                ClientResponse::Direct(msg) => {
                                                    if let Ok(json) = serde_json::to_string(&msg) {
                                                        let _ = tx.send(json);
                                                    }
                                                }
                                                ClientResponse::Broadcast(msg) => {
                                                    if let Ok(json) = serde_json::to_string(&msg) {
                                                        let _ = tx.send(json);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Error handling canvas message: {}", e);
                                        let error_msg = CanvasMessage::Error(e);
                                        if let Ok(json) = serde_json::to_string(&error_msg) {
                                            let _ = tx.send(json); 
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to parse canvas message: {}", e);
                            }
                        }
                    }
                    Message::Close(_) => {break},
                    _ => {}
                }
            }
        })
    };

    // wait for either task to complete
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    // cleanup: remove client from room
    if let Some(manager) = &canvas_manager {
        manager.remove_client_from_room(room_id.clone(), client_id.clone()).await;
    }

    log::info!("Canvas client {} disconnected from room {}", client_id, room_id);
}

#[derive(Debug)]
enum ClientResponse {
    Direct(CanvasMessage),
    Broadcast(CanvasMessage),
}

async fn handle_canvas_message(
    message: CanvasMessage,
    canvas_manager: &Option<CanvasRoomManager>,
    room_id: &str,
    client_id: &str,
) -> Result<Vec<ClientResponse>, String> {
    let manager = canvas_manager.as_ref()
        .ok_or_else(|| "Canvas manager not available".to_string())?;

    match message {
        CanvasMessage::SubmitOperation(operation) => {
            log::debug!("Processing operation from {}: {:?}", client_id, operation.operation_type);

            let (transformed_op, _broadcast_clients) = manager
                .process_operation(room_id.to_string(), operation)
                .await?;

            let mut responses = vec![
                // Send ack to sender
                ClientResponse::Direct(CanvasMessage::OperationAck {
                    operation_id: transformed_op.id.clone(),
                    server_sequence: transformed_op.server_sequence,
                })
            ];

            // broadcast to all other clients (handled by broadcast channel)
            responses.push(ClientResponse::Broadcast(
                CanvasMessage::RemoteOperation(transformed_op)
            ));

            Ok(responses)
        }
        CanvasMessage::RequestState => {
            if let Some(state) = manager.get_room_state(room_id.to_string()).await {
                Ok(vec![ClientResponse::Direct(CanvasMessage::StateSync(state))])
            } else {
                Err("Room state not found".to_string())
            }
        }
        CanvasMessage::SaveCanvas => {
            match manager.save_canvas(room_id.to_string()).await {
                Ok(url) => {
                    Ok(vec![ClientResponse::Direct(CanvasMessage::CanvasSaved { url })])
                }
                Err(e) => Err(format!("Failed to save canvas: {}", e))
            }
        }
        CanvasMessage::Ping => {
            Ok(vec![ClientResponse::Direct(CanvasMessage::Pong)])
        }
        _ => {
            Ok(vec![])
        }
    }
}
