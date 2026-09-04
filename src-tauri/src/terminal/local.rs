use std::{
    env,
    io::{Read, Write},
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::{
    error::{AppError, AppResult},
    types::TerminalSize,
};

/// 实现逻辑是在真正的终端里， shell 以为在跟实际的屏幕对话，但实际是在跟 PTY
/// 程序 <-> master <-> slave <-> bash/zsh(child)
/// 之所以都为 Box<dyn .. + Send> 是因为 portable_pty 在不同的操作系统上实现的逻辑不一样，库只暴露 trait
/// Box<dyn Trait> = 堆上放一个「满足这个接口的具体东西」，编译期不知道具体是哪家实现

pub struct LocalPty {
    // 从 master 读 子进程打出来的字
    pub reader: Box<dyn Read + Send>,

    // 往 master 写
    pub writer: Box<dyn Write + Send>,

    // shell
    pub child: Box<dyn Child + Send + Sync>,

    // pty 本身
    pub master: Box<dyn MasterPty + Send>,
}

impl LocalPty {
    pub fn spawn(size: TerminalSize) -> AppResult<Self> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: size.rows,
                cols: size.columns,
                pixel_height: 0,
                pixel_width: 0,
            })
            .map_err(|_| AppError::internal())?;

        // CommandBuilder 告诉 portable_pty 在 slave 端启动哪个进程，并把它的 stdin/out/err 接到这块 PTY 上
        // 这段表明在 slave 端启动 Shell
        let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
        let mut command = CommandBuilder::new(shell);

        command.arg("-i");
        command.env("TERM", "xterm-256color");
        command.env("TERM_PROGRAM", "Porthole");

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|_| AppError::local_terminal_unavailable())?;

        // 子进程起来后，父进程应关掉自己手里的 slave。留着可能导致：
        // - 子进程以为还有人握着终端
        // - 或退出检测/挂断行为怪异
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| AppError::internal())?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|_| AppError::internal())?;

        Ok(Self {
            reader,
            writer,
            child,
            master: pair.master,
        })
    }
}
