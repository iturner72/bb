use super::types::*;
use cfg_if::cfg_if;
use chrono::Utc;
use std::collections::{HashMap, VecDeque};

cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use web_sys::js_sys;
    }
}

#[derive(Debug, Clone)]
pub struct ClientCanvasState {
    pub canvas_state: CanvasState,
    pub pending_operations: VecDeque<Operation>,
    pub client_sequence: u64,
    pub last_server_sequence: u64,
    pub client_id: String,
    pub current_stroke: Option<CurrentStroke>,
    pub redo_stack: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CurrentStroke {
    pub stroke_id: String,
    pub points: Vec<Point>,
    pub color: String,
    pub brush_size: u32,
}

impl ClientCanvasState {
    pub fn new(client_id: String) -> Self {
        Self {
            canvas_state: CanvasState {
                strokes: HashMap::new(),
                operation_history: Vec::new(),
                server_sequence: 0,
            },
            pending_operations: VecDeque::new(),
            client_sequence: 0,
            last_server_sequence: 0,
            client_id,
            current_stroke: None,
            redo_stack: Vec::new(),
        }
    }

    pub fn start_stroke(&mut self, color: String, brush_size: u32, point: Point) -> String {
        let stroke_id = self.generate_stroke_id();

        self.current_stroke = Some(CurrentStroke {
            stroke_id: stroke_id.clone(),
            points: vec![point],
            color,
            brush_size,
        });

        stroke_id
    }

    // add point to current stroke
    pub fn add_to_stroke(&mut self, point: Point) {
        if let Some(ref mut stroke) = self.current_stroke {
            stroke.points.push(point);
        }
    }

    // finish current stroke and create operation
    pub fn finish_stroke(&mut self) -> Option<Operation> {
        if let Some(stroke) = self.current_stroke.take() {
            let operation = self.create_operation(OperationType::DrawStroke {
                stroke_id: stroke.stroke_id,
                points: stroke.points,
                color: stroke.color,
                brush_size: stroke.brush_size,
            });

            // apply immediately for responsive UI
            self.apply_operation(&operation);
            self.pending_operations.push_back(operation.clone());

            Some(operation)
        } else {
            None
        }
    }

    // create delete stroke operation
    pub fn create_delete_stroke(&mut self, stroke_id: String) -> Operation {
        let operation = self.create_operation(OperationType::DeleteStroke { stroke_id });

        // apply immediately for responsive ui
        self.apply_operation(&operation);
        self.pending_operations.push_back(operation.clone());

        operation
    }

    // find stroke at given position for eraser tool
    pub fn find_stroke_at_position(&self, x: f64, y: f64) -> Option<String> {
        // check strokes in reverse order (most recent first)
        for operation in self.canvas_state.operation_history.iter().rev() {
            if let OperationType::DrawStroke { stroke_id, .. } = &operation.operation_type {
                if let Some(stroke) = self.canvas_state.strokes.get(stroke_id) {
                    if !stroke.deleted && self.point_intersects_stroke(x, y, stroke) {
                        return Some(stroke_id.clone());
                    }
                }
            }
        }
        None
    }

    // method to check if a point intersects with a stroke
    fn point_intersects_stroke(&self, x: f64, y: f64, stroke: &Stroke) -> bool {
        let tolerance = stroke.brush_size as f64 / 2.0 + 5.0; // add tolerance to make erasing "easier"

        for point in &stroke.points {
            let dx = x - point.x;
            let dy = y - point.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= tolerance {
                return true;
            }
        }
        false
    }

    // create undo operation
    pub fn create_undo(&mut self, target_operation_id: String) -> Operation {
        let operation = self.create_operation(OperationType::Undo {
            target_operation_id: target_operation_id.clone(),
        });

        self.redo_stack.push(target_operation_id);
        self.apply_operation(&operation);
        self.pending_operations.push_back(operation.clone());

        operation
    }

    // create redo operation
    pub fn create_redo(&mut self) -> Option<Operation> {
        // get the most recent undo that can be redone
        if let Some(undo_target_id) = self.redo_stack.pop() {
            let operation = self.create_operation(OperationType::Redo {
                target_operation_id: undo_target_id,
            });

            self.apply_operation(&operation);
            self.pending_operations.push_back(operation.clone());

            Some(operation)
        } else {
            None
        }
    }

    // create clear operation
    pub fn create_clear(&mut self) -> Operation {
        let operation = self.create_operation(OperationType::Clear);

        self.apply_operation(&operation);
        self.pending_operations.push_back(operation.clone());

        operation
    }

    fn create_operation(&mut self, operation_type: OperationType) -> Operation {
        self.client_sequence += 1;

        Operation {
            id: format!("{}_{}", self.client_id, self.client_sequence),
            client_id: self.client_id.clone(),
            sequence: self.client_sequence,
            server_sequence: self.last_server_sequence,
            operation_type,
            timestamp: Utc::now(),
        }
    }

    // apply operation to local state
    pub fn apply_operation(&mut self, operation: &Operation) {
        match &operation.operation_type {
            OperationType::DrawStroke {
                stroke_id,
                points,
                color,
                brush_size,
            } => {
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

                // clear redo stack when new operation is performed
                if operation.client_id == self.client_id {
                    self.redo_stack.clear();
                }
            }
            OperationType::DeleteStroke { stroke_id } => {
                if let Some(stroke) = self.canvas_state.strokes.get_mut(stroke_id) {
                    stroke.deleted = true;
                }

                // clear redo stack when new operation is performed
                if operation.client_id == self.client_id {
                    self.redo_stack.clear();
                }
            }
            OperationType::Clear => {
                for stroke in self.canvas_state.strokes.values_mut() {
                    stroke.deleted = true;
                }

                // clear redo stack when new operation is performed
                if operation.client_id == self.client_id {
                    self.redo_stack.clear();
                }
            }
            OperationType::Undo {
                target_operation_id,
            } => {
                // find the operation to undo and reverse its effects
                if let Some(target_op) = self
                    .canvas_state
                    .operation_history
                    .iter()
                    .find(|op| op.id == *target_operation_id)
                {
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
            OperationType::Redo { target_operation_id } => {
                // find the original operation that was undone and restore its effects
                if let Some(target_op) = self
                    .canvas_state
                    .operation_history
                    .iter()
                    .find(|op| op.id == *target_operation_id)
                {
                    match &target_op.operation_type {
                        OperationType::DrawStroke { stroke_id, .. } => {
                            if let Some(stroke) = self.canvas_state.strokes.get_mut(stroke_id) {
                                stroke.deleted = false;
                            }
                        } 
                        OperationType::DeleteStroke { stroke_id } => {
                            if let Some(stroke) = self.canvas_state.strokes.get_mut(stroke_id) {
                                stroke.deleted = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    
        self.canvas_state.operation_history.push(operation.clone());
    }

    // handle server acknowledgement
    pub fn handle_server_ack(&mut self, operation_id: &str, server_sequence: u64) {
        self.pending_operations.retain(|op| op.id != operation_id);
        self.last_server_sequence = server_sequence;
    }

    // handle operation from another client
    pub fn handle_remote_operation(&mut self, mut operation: Operation) -> Operation {
        // transform against all pending operations
        for pending_op in &self.pending_operations {
            operation = transform_operation(operation, pending_op, TransformSide::Left);
        }

        // apply the transformed operation
        self.apply_operation(&operation);
        self.last_server_sequence = operation.server_sequence;

        operation
    }

    // sync with server state
    pub fn sync_with_server_state(&mut self, server_state: CanvasState) {
        self.canvas_state = server_state;
        self.last_server_sequence = self.canvas_state.server_sequence;

        // clear pending operations that are now in server state
        let server_op_ids: std::collections::HashSet<String> = self
            .canvas_state
            .operation_history
            .iter()
            .map(|op| op.id.clone())
            .collect();

        self.pending_operations
            .retain(|op| !server_op_ids.contains(&op.id));
    }

    // get visible strokes for rendering
    pub fn get_visible_strokes(&self) -> Vec<&Stroke> {
        let mut strokes = Vec::new();

        // iterate through operation history to get drawing order
        for op in &self.canvas_state.operation_history {
            if let OperationType::DrawStroke { stroke_id, .. } = &op.operation_type {
                if let Some(stroke) = self.canvas_state.strokes.get(stroke_id) {
                    if !stroke.deleted {
                        strokes.push(stroke);
                    }
                }
            }
        }

        strokes
    }

    // Get undoable operations - only DrawStroke operations that resulted in real visible strokes
    pub fn get_undoable_operations(&self) -> Vec<&Operation> {
        // Find DrawStroke operations by this client that currently have real visible strokes
        let undoable_ops: Vec<&Operation> = self.canvas_state.operation_history
            .iter()
            .filter(|op| {
                // Must be from this client
                if op.client_id != self.client_id {
                    return false;
                }
                
                // Must be a DrawStroke operation
                if let OperationType::DrawStroke { stroke_id, .. } = &op.operation_type {
                    // The stroke must exist, not be deleted, and not be a no-op
                    if let Some(stroke) = self.canvas_state.strokes.get(stroke_id) {
                        !stroke.deleted && 
                        !stroke_id.starts_with("noop_") && // Filter out no-op strokes
                        !stroke.points.is_empty() && // Filter out empty strokes
                        stroke.color != "transparent" // Filter out transparent strokes
                    } else {
                        false
                    }
                } else {
                    // Never allow undoing of non-DrawStroke operations
                    false
                }
            })
            .collect();
        
        undoable_ops
    }

    pub fn can_undo(&self) -> bool {
        !self.get_undoable_operations().is_empty()
    }

    pub fn get_redoable_operations(&self) -> Vec<String> {
        // return operations in reverse order (most recent undo first)
        self.redo_stack.iter().rev().cloned().collect()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn get_pending_operations(&self) -> Vec<Operation> {
        self.pending_operations.iter().cloned().collect()
    }

    fn generate_stroke_id(&self) -> String {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
                format!("{}_{}", self.client_id, js_sys::Date::now() as u64)
            } else {
                format!("{}_{}", self.client_id, self.client_sequence + 1)
            }
        }
    }
}
