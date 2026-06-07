use cc_switch_lib::remote::{
    parse_session_response_line, RemoteCommandError, RemoteSessionError, RemoteSessionManager,
    RemoteSessionState,
};
use serde_json::json;

#[test]
fn parses_session_success_response() {
    let response =
        parse_session_response_line(r#"{"id":"1","ok":true,"data":{"value":42},"error":null}"#)
            .expect("response");

    assert_eq!(response.id, "1");
    assert!(response.ok);
    assert_eq!(response.data, Some(json!({"value":42})));
}

#[test]
fn parses_session_error_response() {
    let response = parse_session_response_line(
        r#"{"id":"2","ok":false,"data":null,"error":{"code":"unsupported_command","message":"no"}}"#,
    )
    .expect("response");

    assert_eq!(response.id, "2");
    assert!(!response.ok);
    assert_eq!(
        response.error,
        Some(RemoteCommandError {
            code: "unsupported_command".to_string(),
            message: "no".to_string(),
        })
    );
}

#[test]
fn invalid_session_response_is_classified() {
    let error = parse_session_response_line("{").unwrap_err();

    assert!(matches!(error, RemoteSessionError::InvalidJson(_)));
}

#[tokio::test]
async fn new_manager_reports_idle_for_unknown_profile() {
    let manager = RemoteSessionManager::default();
    let status = manager.status("missing").await;

    assert_eq!(status.profile_id, "missing");
    assert_eq!(status.state, RemoteSessionState::Idle);
    assert_eq!(status.last_error, None);
}
