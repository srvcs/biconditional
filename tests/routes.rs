use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_biconditional::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

/// Mock dependency answering `POST /` with a fixed status + body.
async fn spawn_mock(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Mock `srvcs-implication`: `a -> b` is `(NOT a) OR b`, computed from the
/// request body so both call directions are answered correctly.
async fn spawn_implication() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|Json(req): Json<Value>| async move {
            let a = req["a"].as_bool().unwrap_or(false);
            let b = req["b"].as_bool().unwrap_or(false);
            let result = !a || b;
            Json(json!({ "a": a, "b": b, "result": result }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Mock `srvcs-and`: `a AND b`, computed from the request body.
async fn spawn_and() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|Json(req): Json<Value>| async move {
            let a = req["a"].as_bool().unwrap_or(false);
            let b = req["b"].as_bool().unwrap_or(false);
            let result = a && b;
            Json(json!({ "a": a, "b": b, "result": result }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(implication_url: &str, and_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            implication_url: implication_url.to_string(),
            and_url: and_url.to_string(),
        },
    )
}

async fn eval(implication_url: &str, and_url: &str, a: Value, b: Value) -> (StatusCode, Value) {
    let res = app(implication_url, and_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": a, "b": b }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

const DEAD_URL: &str = "http://127.0.0.1:1";

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL, DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn index_ok() {
    let res = app(DEAD_URL, DEAD_URL)
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "srvcs-biconditional");
    assert_eq!(
        body["depends_on"],
        json!(["srvcs-implication", "srvcs-and"])
    );
}

// Truth table: a <-> b is true exactly when a == b.

#[tokio::test]
async fn true_iff_true() {
    let imp = spawn_implication().await;
    let and = spawn_and().await;
    let (status, body) = eval(&imp, &and, json!(true), json!(true)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["a"], true);
    assert_eq!(body["b"], true);
    assert_eq!(body["result"], true);
}

#[tokio::test]
async fn true_iff_false() {
    let imp = spawn_implication().await;
    let and = spawn_and().await;
    let (status, body) = eval(&imp, &and, json!(true), json!(false)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
}

#[tokio::test]
async fn false_iff_true() {
    let imp = spawn_implication().await;
    let and = spawn_and().await;
    let (status, body) = eval(&imp, &and, json!(false), json!(true)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], false);
}

#[tokio::test]
async fn false_iff_false() {
    let imp = spawn_implication().await;
    let and = spawn_and().await;
    let (status, body) = eval(&imp, &and, json!(false), json!(false)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], true);
}

#[tokio::test]
async fn degrades_when_implication_is_unreachable() {
    let and = spawn_and().await;
    let (status, body) = eval(DEAD_URL, &and, json!(true), json!(true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-implication");
}

#[tokio::test]
async fn degrades_when_and_is_unreachable() {
    let imp = spawn_implication().await;
    let (status, body) = eval(&imp, DEAD_URL, json!(true), json!(true)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-and");
}

#[tokio::test]
async fn forwards_invalid_input_from_implication() {
    let imp = spawn_mock(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "operand is not a boolean" }),
    )
    .await;
    let and = spawn_and().await;
    let (status, body) = eval(&imp, &and, json!("nope"), json!(true)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "operand is not a boolean");
}

#[tokio::test]
async fn forwards_invalid_input_from_and() {
    // Implication succeeds, but `and` rejects the input.
    let imp = spawn_implication().await;
    let and = spawn_mock(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "operand is not a boolean" }),
    )
    .await;
    let (status, body) = eval(&imp, &and, json!(true), json!(true)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "operand is not a boolean");
}
