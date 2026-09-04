use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error, Serialize)]
#[error("{code}: {message}")]
pub struct AppError {
    /// 机器可判断的稳定错误码，例如 invalid_input
    pub code: &'static str,

    /// 给用户看的安全错误说明
    pub message: String,
}

impl AppError {
    /// 接受一个任何能够转换为 String 的类型
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_input",
            message: message.into(),
        }
    }

    /// 请求的会话 UUID 不存在
    pub fn session_not_found() -> Self {
        Self {
            code: "session_not_found",
            message: "The terminal session was not found.".to_owned(),
        }
    }

    /// 会话已注册，但终端后端尚未启动
    pub fn session_not_ready() -> Self {
        Self {
            code: "session_not_ready",
            message: "The terminal session is not ready yet.".to_owned(),
        }
    }

    pub fn internal() -> Self {
        Self {
            code: "internal",
            message: "The terminal backend encountered an internal error.".to_owned(),
        }
    }

    pub fn local_terminal_unavailable() -> Self {
        Self {
            code: "local_terminal_unavailable",
            message: "The local terminal could not be started.".to_owned(),
        }
    }
}
