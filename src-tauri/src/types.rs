use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// 限制终端的最大列数和行数，避免异常请求消耗资源。
pub const MAX_TERMINAL_DIMENSION: u16 = 500;

/// 单次写入终端的最大原始字节数，防止输入造成无界内存占用。
pub const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
}

/// 终端会话连接接收到的目标类型
///
/// 目前只有 PTY 与 远程 SSH
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Local,
}

/// 终端会话当前的生命周期状态
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStatus {
    Opened,
    Closed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStarted {
    /// UUID 用于后续输入、调整尺寸和关闭操作
    pub session_id: Uuid,

    /// 前端可据此知道会话来自本机还是 SSH
    pub kind: SessionKind,
}

/// 前端写入某个终端会话的请求。
///
/// data_base64 是原始终端字节的 Base64 表示
/// 前端 Uint8Array -> base64 编码 -> IPC JSON -> Rust 解码
///
/// Tauri IPC 使用 JSON，JSON 不适合传输安全字节，因此不能直接把任意字节放进 JSON 字符串
/// 没有使用 Debug Trait 因为输入的内容可能包含命令，甚至密码，存在泄漏风险
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInputRequest {
    /// 会话的唯一 id
    pub session_id: Uuid,

    /// 编码后的终端字节
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalDataEvent {
    pub session_id: Uuid,
    pub data_base64: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStatusEvent {
    pub session_id: Uuid,
    pub state: TerminalStatus,

    /// 正常 opened / closed 不需要消息；
    /// 未来 failed 事件才会带安全错误文案。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl TerminalSize {
    /// 验证尺寸安全可以用于 PTY 与远程 SSH PTY
    pub fn validate(self) -> AppResult<Self> {
        // u16 不会是负数，但仍可能为 0
        if self.columns == 0 || self.columns > MAX_TERMINAL_DIMENSION {
            return Err(AppError::invalid_input(format!(
                "Terminal columns must be between 1 and {MAX_TERMINAL_DIMENSION}."
            )));
        }

        if self.rows == 0 || self.rows > MAX_TERMINAL_DIMENSION {
            return Err(AppError::invalid_input(format!(
                "Terminal rows must be between 1 and {MAX_TERMINAL_DIMENSION}."
            )));
        }

        // 返回 self 后续在调用的时候可以 let size = size.validate()?
        Ok(self)
    }
}

impl TerminalInputRequest {
    /// 解码前端传来的 Base64，并限制真实字节大小。
    ///
    /// 消费 self：调用完成后，原始 Base64 字符串会尽早离开当前作用域。
    /// STANDARD 是 base64 crate 提供的一个标准 Base64 编码规则实例
    pub fn decode(self) -> AppResult<(Uuid, Vec<u8>)> {
        let bytes = STANDARD
            // 消费 data_base64 的所有权避免被其他位置引用
            .decode(self.data_base64)
            // map_err 接受闭包 把原始 Err 转换为另一个错误类型
            .map_err(|_| AppError::invalid_input("Terminal input must be valid Base64."))?;

        if bytes.len() > MAX_INPUT_BYTES {
            return Err(AppError::invalid_input(format!(
                "Terminal input cannot exceed {MAX_INPUT_BYTES} bytes."
            )));
        }

        Ok((self.session_id, bytes))
    }
}

#[cfg(test)]
#[path = "tests/types.rs"]
mod test;
