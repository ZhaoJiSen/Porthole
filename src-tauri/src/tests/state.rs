use super::*;
use tokio::sync::mpsc;

fn test_handle() -> SessionHandle {
    let (input, _receiver) = mpsc::channel(1);
    SessionHandle::new(SessionKind::Local, input)
}

#[tokio::test]
async fn manager_returns_the_registered_session() {
    let manager = SessionManager::default();
    let session_id = Uuid::new_v4();
    let handle = test_handle();

    manager.insert(session_id, handle).await;

    let session = manager
        .get(session_id)
        .await
        .expect("registered session should exist");
    assert_eq!(session.kind, SessionKind::Local)
}

#[tokio::test]
async fn manager_removes_registered_session() {
    let manager = SessionManager::default();
    let session_id = Uuid::new_v4();
    let handle = test_handle();

    manager.insert(session_id, handle).await;

    let removed = manager
        .close(session_id)
        .await
        .expect("registered session should be removable");

    assert_eq!(removed.kind, SessionKind::Local);
    assert!(removed.cancellation.is_cancelled());
    assert!(manager.get(session_id).await.is_none());
}

#[tokio::test]
async fn session_forwards_terminal_input() {
    let (input, mut receiver) = mpsc::channel(1);
    let handle = SessionHandle::new(SessionKind::Local, input);

    handle
        .send_input(b"pwd\r".to_vec())
        .await
        .expect("open input channel should accept bytes");

    assert_eq!(receiver.recv().await, Some(b"pwd\r".to_vec()));
}
