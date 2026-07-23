use std::time::Duration;

use rustglm::{RealtimeClient, RealtimeSession};
use tokio::time::timeout;

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY and performs a live Realtime connection"]
async fn live_realtime_session() {
    let api_key = std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY is required");
    let mut connection = timeout(Duration::from_secs(20), RealtimeClient::connect(api_key))
        .await
        .expect("Realtime connection timed out")
        .expect("Realtime connection must succeed");
    let event = timeout(Duration::from_secs(20), connection.next_event())
        .await
        .expect("session.created timed out")
        .expect("Realtime connection closed before session.created")
        .expect("session.created must decode");
    assert_eq!(event.event_type, "session.created");
    connection
        .sender()
        .update_session(RealtimeSession::default())
        .await
        .expect("session.update must send");
    connection.close().await.expect("connection must close");
}
