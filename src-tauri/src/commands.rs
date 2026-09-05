use crate::terminal::local::LocalPty;
use crate::types::TerminalStatus;
use crate::{
    error::{AppError, AppResult},
    events,
    state::{AppState, SessionHandle},
    types::{SessionKind, SessionStarted, TerminalInputRequest, TerminalSize},
};
use tauri::State;
use tokio::sync::mpsc;
use uuid::Uuid;

/// State 是 Tauri 注入给 command 的全局共享应用状态
#[tauri::command]
pub async fn create_local_session(
    state: State<'_, AppState>,
    size: TerminalSize,
) -> AppResult<SessionStarted> {
    let size = size.validate()?;
    let session_id = Uuid::new_v4();

    let local_pty = LocalPty::spawn(size)?;
    let (tx, rx) = mpsc::channel(64);
    let handle = SessionHandle::new(SessionKind::Local, tx);
    let cancellation = handle.cancellation_token();
    let kind = handle.kind;

    state.sessions.insert(session_id, handle).await;

    // 开启一个会话的连接
    events::emit_terminal_status(&state.app_handle, session_id, TerminalStatus::Opened, None)?;

    // 实际要去写入数据了，要把接受者 rx 传入。tx 被 handler 发送到了前端，前端就可以写入内容了
    local_pty.start_workers(
        state.app_handle.clone(),
        state.sessions.clone(),
        session_id,
        rx,
        cancellation,
    );

    Ok(SessionStarted { session_id, kind })
}

#[tauri::command]
pub async fn write_terminal_input(
    state: State<'_, AppState>,
    request: TerminalInputRequest,
) -> AppResult<()> {
    let (session_id, bytes) = request.decode()?;

    let handle = state
        .sessions
        .get(session_id)
        .await
        .ok_or_else(AppError::session_not_found)?;

    handle.send_input(bytes).await
}

/// 关闭一个终端会话
#[tauri::command]
pub async fn close_terminal_session(state: State<'_, AppState>, session_id: Uuid) -> AppResult<()> {
    state
        .sessions
        .close(session_id)
        .await
        .ok_or_else(AppError::session_not_found)?;

    events::emit_terminal_status(&state.app_handle, session_id, TerminalStatus::Closed, None)?;

    Ok(())
}
