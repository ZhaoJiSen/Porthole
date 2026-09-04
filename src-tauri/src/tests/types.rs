use super::*;

#[test]
fn terminal_size_rejects_zero_columns() {
    let error = TerminalSize {
        columns: 0,
        rows: 24,
    }
    .validate()
    .expect_err("zero columns should be invalid");

    assert_eq!(error.code, "invalid_input");
}

#[test]
fn terminal_size_rejects_zero_rows() {
    let error = TerminalSize {
        columns: 24,
        rows: 0,
    }
    .validate()
    .expect_err("zero rows should be invalid");

    assert_eq!(error.code, "invalid_input");
}

#[test]
fn terminal_input_rejects_invalid_base64() {
    let error = TerminalInputRequest {
        session_id: Uuid::nil(),
        data_base64: "%%%".to_owned(),
    }
    .decode()
    .expect_err("invalid Base64 should be rejected");

    assert_eq!(error.code, "invalid_input")
}

#[test]
fn terminal_input_rejects_oversized_payload() {
    let payload = STANDARD.encode(vec![0_u8; MAX_INPUT_BYTES + 1]);

    let error = TerminalInputRequest {
        session_id: Uuid::nil(),
        data_base64: payload,
    }
    .decode()
    .expect_err("oversized input should be rejected");

    assert_eq!(error.code, "invalid_input");
}

#[test]
fn terminal_input_decodes_valid_bytes_for_the_same_session() {
    let session_id = Uuid::new_v4();

    let payload = STANDARD.encode(b"ls\r");
    let (decoded_session_id, bytes) = TerminalInputRequest {
        session_id,
        data_base64: payload,
    }
    .decode()
    .expect("valid Base64 should decode");

    assert_eq!(decoded_session_id, session_id);
    assert_eq!(bytes, b"ls\r");
}
