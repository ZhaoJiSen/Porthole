use std::{collections::HashMap, sync::Arc};
use tauri::AppHandle;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    types::SessionKind,
};

/// 一个已注册终端会话的最小状态。
///
/// 现在只记录类型；后面会逐步加入 PTY / SSH 的控制通道
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub kind: SessionKind,

    /// 所有属于该会话的后台任务共享一个取消信号
    cancellation: CancellationToken,

    // 发送者
    input: Option<mpsc::Sender<Vec<u8>>>,
}

/// 全部活动终端会话的注册表。
///
/// Arc：多个 Tauri command 共享同一个注册表。
/// RwLock：允许多个读取操作并发，但插入/删除时独占写锁。
#[derive(Debug, Default)]
pub struct SessionManager {
    pub sessions: Arc<RwLock<HashMap<Uuid, SessionHandle>>>,
}

/// 注入 Tauri 的全局应用状态。
///
/// 每个 Tauri command 都能通过 State<AppState>
/// 访问同一份 SessionManager。
pub struct AppState {
    /// 全部活动会话
    pub sessions: SessionManager,

    /// 用于从后台逻辑向前端发送终端事件
    pub app_handle: AppHandle,
}

impl SessionManager {
    /// 注册一个新会话
    pub async fn insert(&self, session_id: Uuid, handle: SessionHandle) {
        self.sessions.write().await.insert(session_id, handle);
    }

    /// 按 UUID 查找会话
    pub async fn get(&self, session_id: Uuid) -> Option<SessionHandle> {
        // cloned 专门用于容器里装的是引用的情况，把内部引用指向的值 clone 出来
        // HashMap get 返回 Option<&SessionHandle> 而需要的是 Option<SessionHandle>
        self.sessions.read().await.get(&session_id).cloned()
    }

    /// 移除会话
    /// 会话关闭后，先通知后台取消任务在调用 remove 从注册表中移除
    pub async fn close(&self, session_id: Uuid) -> Option<SessionHandle> {
        let handle = self.sessions.write().await.remove(&session_id)?;

        handle.cancel();
        Some(handle)
    }
}

impl SessionHandle {
    /// 创建一个新会话及其独立取消信号
    pub fn new(kind: SessionKind) -> Self {
        Self {
            kind,
            input: None,
            cancellation: CancellationToken::new(),
        }
    }

    /// 请求该会话的所有后台任务停止
    pub fn cancel(&self) {
        self.cancellation.cancel()
    }

    pub fn with_input(kind: SessionKind, input: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            kind,
            cancellation: CancellationToken::new(),
            input: Some(input),
        }
    }

    pub async fn send_input(&self, bytes: Vec<u8>) -> AppResult<()> {
        let input = self
            .input
            .as_ref()
            .ok_or_else(AppError::session_not_ready)?;

        input
            .send(bytes)
            .await
            .map_err(|_| AppError::session_not_ready())
    }
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            sessions: SessionManager::default(),
        }
    }
}

#[cfg(test)]
#[path = "tests/state.rs"]
mod test;
