use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    io::Read,
    thread,
    time::{Duration, Instant},
};

/// - PtySystem：创建当前系统原生 PTY 的工厂。
/// - PtyPair：一对 master/slave 终端端点。
/// - slave：启动 shell 的一侧；shell 会以为自己连接在真实终端上。
/// - master：Porthole 控制的一侧；以后 xterm 输入写到 writer，shell 输出从 reader 读取。

#[test]
fn local_pty_returns_child_output() {
    // 根据当前系统生成原生的 pty 实现
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("PTY should open");

    // 这里不是交互 shell；测试只执行一个固定命令并退出，
    // 避免测试卡在等待人工输入。
    let mut command = CommandBuilder::new("/usr/bin/printf");
    command.arg("porthole-pty");

    let mut child = pair
        .slave
        .spawn_command(command)
        .expect("shell should start in pty");

    // 子进程启动后不需要再 slave 端
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .expect("PTY reader should be available");

    let expected = b"porthole-pty";
    let mut output = Vec::new();
    let mut buffer = [0_u8; 64];

    while !output
        .windows(expected.len())
        .any(|window| window == expected)
    {
        let byte_count = reader
            .read(&mut buffer)
            .expect("PTY output should be readable");

        // EOF：子进程已关闭输出，但还没读到目标内容。
        if byte_count == 0 {
            break;
        }

        output.extend_from_slice(&buffer[..byte_count]);
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        match child.try_wait().expect("child status should be readable") {
            Some(status) => break status,

            None if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }

            None => {
                child.kill().expect("timed out child should be terminated");
                panic!("PTY child did not exit within two seconds");
            }
        }
    };

    assert!(status.success());

    assert!(
        output
            .windows(expected.len())
            .any(|window| window == expected),
        "PTY output should contain the expected bytes: {output:?}"
    );
}
