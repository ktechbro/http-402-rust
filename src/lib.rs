//! A teaching simulation of the HTTP 402 challenge/retry lifecycle.
//! The custom `demo` scheme is intentionally not a blockchain payment scheme.
use axum::{
    Json, Router,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/premium", get(premium))
}

async fn index() -> Json<Value> {
    Json(json!({
        "tutorial": "1 — Understand HTTP 402",
        "mode": "local simulation — no real payments",
        "instructions": "GET /premium, decode PAYMENT-REQUIRED, then run docker compose run --rm demo",
        "public": "/health",
        "protected": "/premium"
    }))
}

fn resource() -> Value {
    json!({
        "url": "http://127.0.0.1:3000/premium",
        "description": "A local teaching resource; no real payment is accepted",
        "mimeType": "application/json"
    })
}

fn requirements() -> Value {
    json!({
        "scheme": "demo",
        "network": "demo:local",
        "amount": "1",
        "asset": "DEMO",
        "payTo": "tutorial-server",
        "maxTimeoutSeconds": 60
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemoPayment {
    x402_version: u8,
    resource: Value,
    accepted: Value,
    payload: DemoPayload,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemoPayload {
    demo_token: String,
}

async fn premium(headers: HeaderMap) -> Response {
    let error = match headers.get("payment-signature") {
        None => "PAYMENT-SIGNATURE header is required",
        Some(header) => {
            let payment = header
                .to_str()
                .ok()
                .and_then(|s| STANDARD.decode(s).ok())
                .and_then(|bytes| serde_json::from_slice::<DemoPayment>(&bytes).ok());
            match payment {
                Some(p)
                    if p.x402_version == 2
                        && p.resource == resource()
                        && p.accepted == requirements()
                        && p.payload.demo_token == "tutorial-only" =>
                {
                    return no_store(
                        Json(json!({
                            "data": "You unlocked the tutorial's protected resource!",
                            "simulation": true,
                            "paymentStatus": "simulated — no funds moved"
                        }))
                        .into_response(),
                    );
                }
                _ => {
                    "Invalid demo payment; use the advertised requirements and tutorial-only token"
                }
            }
        }
    };

    let challenge = json!({
        "x402Version": 2,
        "error": error,
        "resource": resource(),
        "accepts": [requirements()]
    });
    // The header is the canonical challenge; the body mirrors it for readability.
    let encoded = STANDARD.encode(serde_json::to_vec(&challenge).expect("JSON value serializes"));
    let mut response = (StatusCode::PAYMENT_REQUIRED, Json(challenge)).into_response();
    response.headers_mut().insert(
        "payment-required",
        HeaderValue::from_str(&encoded).expect("base64 is a valid HTTP header"),
    );
    no_store(response)
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response
}
