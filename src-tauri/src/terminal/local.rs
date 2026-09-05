use std::{
    env,
    io::{Read, Write},
    thread,
    time::Duration,
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    events,
    state::SessionManager,
    types::{TerminalSize, TerminalStatus},
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

    pub fn start_workers(
        self,
        app: AppHandle,
        sessions: SessionManager,
        session_id: Uuid,
        mut input: mpsc::Receiver<Vec<u8>>,
        cancellation: CancellationToken,
    ) {
        let Self {
            mut reader,
            mut writer,
            mut child,
            master,
        } = self;

        // 将 channel 中收到的前端输入写入 PTY
        thread::spawn(move || {
            while let Some(bytes) = input.blocking_recv() {
                if writer
                    .write_all(&bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        });

        // 读取 PYT 输出并发送给前端
        let output_app = app.clone();
        let reader_worker = thread::spawn(move || {
            let mut buffer = [0_u8; 16 * 1024];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(byte_count) => {
                        if events::emit_terminal_data(
                            &output_app,
                            session_id,
                            &buffer[..byte_count],
                        )
                        .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        // 监控 shell 退出和会话取消
        thread::spawn(move || {
            loop {
                if cancellation.is_cancelled() {
                    let _ = child.kill();
                }

                // try_wait 就是轮询：try_wait 立刻问一句；None 就歇一会儿再问，Some(status) 说明子进程已经退出，收尾走人
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => {
                        let _ = child.kill();
                        break;
                    }
                }
            }

            drop(master);
            let _ = reader_worker.join();

            tauri::async_runtime::spawn(async move {
                // is_some() 问的是：这个 Option 里有没有值——是 Some(_) 就返回 true，是 None 就 false
                if sessions.close(session_id).await.is_some() {
                    let _ = events::emit_terminal_status(
                        &app,
                        session_id,
                        TerminalStatus::Closed,
                        None,
                    );
                }
            })
        });
    }
}
