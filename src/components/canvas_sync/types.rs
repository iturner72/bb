use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Operation {
    pub id: String,
    pub client_id: String,
    pub sequence: u64,        // client's local sequence number
    pub server_sequence: u64, // server's sequence when this op was created
    pub operation_type: OperationType,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OperationType {
    DrawStroke {
        stroke_id: String,
        points: Vec<Point>,
        color: String,
        brush_size: u32,
    },
    DeleteStroke {
        stroke_id: String,
    },
    Clear,
    Undo {
        target_operation_id: String,
    },
    Redo {
        target_operation_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub pressure: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Stroke {
    pub id: String,
    pub points: Vec<Point>,
    pub color: String,
    pub brush_size: u32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanvasState {
    pub strokes: HashMap<String, Stroke>,
    pub operation_history: Vec<Operation>,
    pub server_sequence: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CanvasMessage {
    // Client -> Server
    SubmitOperation(Operation),
    RequestState,
    SaveCanvas,

    // Server -> Client
    OperationAck {
        operation_id: String,
        server_sequence: u64,
    },
    RemoteOperation(Operation),
    StateSync(CanvasState),
    CanvasSaved {
        url: String,
    },

    // bi-directional
    Ping,
    Pong,
    Error(String),
}

#[derive(Debug, Clone, Copy)]
pub enum TransformSide {
    Left,  // This operation has priority
    Right, // Other operation has priority
}

// core operational transform function - works on both client and server
pub fn transform_operation(op1: Operation, op2: &Operation, side: TransformSide) -> Operation {
    match (&op1.operation_type, &op2.operation_type) {
        // DrawStroke vs DrawStroke - no conflict, operations are independent
        (OperationType::DrawStroke { .. }, OperationType::DrawStroke { .. }) => op1,

        // DeleteStroke vs DrawStroke
        (
            OperationType::DeleteStroke { stroke_id: id1 },
            OperationType::DrawStroke { stroke_id: id2, .. },
        ) => {
            if id1 == id2 {
                match side {
                    TransformSide::Left => op1, // Delete wins
                    TransformSide::Right => {
                        // Drawing wins, transform delete to no-op by creating empty stroke
                        Operation {
                            operation_type: OperationType::DrawStroke {
                                stroke_id: format!("noop_{}", op1.id),
                                points: vec![],
                                color: "transparent".to_string(),
                                brush_size: 0,
                            },
                            ..op1
                        }
                    }
                }
            } else {
                op1 // no conflict
            }
        }

        // DrawStroke vs DeleteStroke
        (
            OperationType::DrawStroke { stroke_id: id1, .. },
            OperationType::DeleteStroke { stroke_id: id2 },
        ) => {
            if id1 == id2 {
                match side {
                    TransformSide::Left => op1, // Draw wins
                    TransformSide::Right => {
                        // Delete wins, transform draw to delete
                        Operation {
                            operation_type: OperationType::DeleteStroke {
                                stroke_id: id1.clone(),
                            },
                            ..op1
                        }
                    }
                }
            } else {
                op1 // no colflict
            }
        }

        // Clear operations
        (OperationType::Clear, _) => op1, // Clear always takes precedence
        (_, OperationType::Clear) => {
            // transform any operation against clear becomes no-op
            Operation {
                operation_type: OperationType::DrawStroke {
                    stroke_id: format!("noop_{}", op1.id),
                    points: vec![],
                    color: "transparent".to_string(),
                    brush_size: 0,
                },
                ..op1
            }
        }

        // Undo operations
        (
            OperationType::Undo {
                target_operation_id: target1,
            },
            OperationType::Undo {
                target_operation_id: target2,
            },
        ) => {
            if target1 == target2 {
                // both trying to undo the same operation
                match side {
                    TransformSide::Left => op1,
                    TransformSide::Right => {
                        // transform to no-op
                        Operation {
                            operation_type: OperationType::DrawStroke {
                                stroke_id: format!("noop_{}", op1.id),
                                points: vec![],
                                color: "transparent".to_string(),
                                brush_size: 0,
                            },
                            ..op1
                        }
                    }
                }
            } else {
                op1 // different targets, no conflict
            }
        }

        // Redo operations //
        // Redo vs Redo conflicts
        (
            OperationType::Redo {
                target_operation_id: target1,
            },
            OperationType::Redo {
                target_operation_id: target2,
            },
        ) => {
            if target1 == target2 {
                // both trying to redo the same operation
                match side {
                    TransformSide::Left => op1,
                    TransformSide::Right => {
                        // transform to no-op
                        Operation {
                            operation_type: OperationType::DrawStroke {
                                stroke_id: format!("noop_{}", op1.id),
                                points: vec![],
                                color: "transparent".to_string(),
                                brush_size: 0,
                            },
                            ..op1
                        }
                    }
                }
            } else {
                op1 // different targets, no conflict
            }
        }

        // Undo vs Redo conflicts
        (
            OperationType::Undo {
                target_operation_id: undo_target,
            },
            OperationType::Redo {
                target_operation_id: redo_target,
            },
        ) => {
            if undo_target == redo_target {
                // undo and redo targeting the same operation - they cancel out
                match side {
                    TransformSide::Left => op1, // undo wins
                    TransformSide::Right => {
                        // transform undo to no-op since redo happened first
                        Operation {
                            operation_type: OperationType::DrawStroke {
                                stroke_id: format!("noop_{}", op1.id),
                                points: vec![],
                                color: "transparent".to_string(),
                                brush_size: 0,
                            },
                            ..op1
                        }
                    }
                }
            } else {
                op1 // different targets, no conflict
            }
        }

        // Redo vs Undo (reverse of above)
        (
            OperationType::Redo {
                target_operation_id: redo_target,
            },
            OperationType::Undo {
                target_operation_id: undo_target,
            },
        ) => {
            if redo_target == undo_target {
                match side {
                    TransformSide::Left => op1, // redo wins
                    TransformSide::Right => {
                        // transform redo to op-op since undo happened first
                        Operation {
                            operation_type: OperationType::DrawStroke {
                                stroke_id: format!("noop_{}", op1.id),
                                points: vec![],
                                color: "transparent".to_string(),
                                brush_size: 0,
                            },
                            ..op1
                        }
                    }
                }
            } else {
                op1 // different targets, no conflict
            }
        }

        // for all other combinations with redo, no conflicts
        _ => op1,
    }
}
