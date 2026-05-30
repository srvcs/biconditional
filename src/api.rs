use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-biconditional";
pub const CONCERN: &str = "logic: a if and only if b";
pub const DEPENDS_ON: &[&str] = &["srvcs-implication", "srvcs-and"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub implication_url: String,
    pub and_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    #[schema(value_type = Object)]
    pub a: Value,
    #[schema(value_type = Object)]
    pub b: Value,
}

#[derive(Serialize, ToSchema)]
pub struct BiconditionalResponse {
    #[schema(value_type = Object)]
    pub a: Value,
    #[schema(value_type = Object)]
    pub b: Value,
    pub result: bool,
}

fn ok(a: Value, b: Value, result: bool) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "a": a, "b": b, "result": result })),
    )
        .into_response()
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

/// Forward a dependency's response verbatim (used to propagate `422` for invalid
/// input from a leaf dependency).
fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// Ask one dependency for its boolean verdict, mapping its failures to the
/// response this service should return.
async fn ask(url: &str, payload: &Value, dependency: &str) -> Result<bool, Response> {
    match client::call(url, payload).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => Ok(body.get("result").and_then(Value::as_bool).unwrap_or(false)),
        // Invalid input — forward the leaf's rejection unchanged.
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — does `a` hold if and only if `b` holds?
///
/// This service does no logic of its own. A biconditional `a <-> b` is the
/// conjunction of the two implications: `(a -> b) AND (b -> a)`. It asks
/// `srvcs-implication` for each direction, then asks `srvcs-and` to combine
/// them.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = BiconditionalResponse),
        (status = 422, description = "operand is not a valid boolean (forwarded)"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    // a -> b
    let ab = match ask(
        &deps.implication_url,
        &json!({ "a": req.a, "b": req.b }),
        "srvcs-implication",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // b -> a
    let ba = match ask(
        &deps.implication_url,
        &json!({ "a": req.b, "b": req.a }),
        "srvcs-implication",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // (a -> b) AND (b -> a)
    let result = match ask(&deps.and_url, &json!({ "a": ab, "b": ba }), "srvcs-and").await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    ok(req.a, req.b, result)
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, BiconditionalResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_both_dependencies() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-biconditional");
        assert_eq!(info.depends_on, vec!["srvcs-implication", "srvcs-and"]);
    }
}
