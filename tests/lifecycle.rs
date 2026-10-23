use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use http_402_rust::app;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn request(payment: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().uri("/premium");
    if let Some(payment) = payment {
        request = request.header("PAYMENT-SIGNATURE", payment);
    }
    app()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn challenge() -> Value {
    let response = request(None).await;
    assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let encoded = response.headers()["payment-required"].to_str().unwrap();
    let header: Value = serde_json::from_slice(&STANDARD.decode(encoded).unwrap()).unwrap();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(header, body);
    assert_eq!(body["x402Version"], 2);
    assert!(body.get("data").is_none());
    body
}

fn encode(payment: &Value) -> String {
    STANDARD.encode(serde_json::to_vec(payment).unwrap())
}

#[tokio::test]
async fn public_health_needs_no_payment() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn unpaid_request_gets_a_challenge_without_protected_data() {
    challenge().await;
}

#[tokio::test]
async fn malformed_payments_remain_locked() {
    for invalid in ["", "not base64!", "e30=", "//8="] {
        let response = request(Some(invalid)).await;
        assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);
        assert!(response.headers().contains_key("payment-required"));
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert!(body.get("data").is_none());
    }
}

#[tokio::test]
async fn retry_unlocks_only_matching_demo_payments() {
    let challenge = challenge().await;
    let payment = json!({
        "x402Version": 2,
        "resource": challenge["resource"],
        "accepted": challenge["accepts"][0],
        "payload": {"demoToken": "tutorial-only"}
    });
    for pointer in [
        "/x402Version",
        "/resource/url",
        "/accepted/amount",
        "/accepted/payTo",
        "/payload/demoToken",
    ] {
        let mut invalid = payment.clone();
        *invalid.pointer_mut(pointer).unwrap() = json!("wrong");
        assert_eq!(
            request(Some(&encode(&invalid))).await.status(),
            StatusCode::PAYMENT_REQUIRED
        );
    }
    let response = request(Some(&encode(&payment))).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(!response.headers().contains_key("payment-response"));
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(body["simulation"], true);
    assert!(body["data"].is_string());
}
