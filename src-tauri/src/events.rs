use crate::error::{AppError, AppResult};
use crate::types::{TerminalDataEvent, TerminalStatus, TerminalStatusEvent};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub const TERMINAL_STATUS_EVENT: &str = "terminal://status";
pub const TERMINAL_DATA_EVENT: &str = "terminal://data";

pub fn emit_terminal_status(
    app: &AppHandle,
    session_id: Uuid,
    state: TerminalStatus,
    message: Option<String>,
) -> AppResult<()> {
    app.emit(
        TERMINAL_STATUS_EVENT,
        TerminalStatusEvent {
            session_id,
            state,
            message,
        },
    )
    .map_err(|_| AppError::internal())
}

pub fn emit_terminal_data(app: &AppHandle, session_id: Uuid, bytes: &[u8]) -> AppResult<()> {
    app.emit(
        TERMINAL_DATA_EVENT,
        TerminalDataEvent {
            session_id,
            data_base64: STANDARD.encode(bytes),
        },
    )
    .map_err(|_| AppError::internal())
}
