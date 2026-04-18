mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        world: "http-api",
        generate_all,
    });
}

use bindings::wasmcloud::messaging::consumer;

use wstd::{
    http::{Body, Request, Response, StatusCode},
    time::Duration,
};

static UI_HTML: &str = include_str!("../ui.html");
static TRANSFORMS_JSON: &str = include_str!("../transforms.json");

#[wstd::http_server]
async fn main(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    match (req.method().as_str(), req.uri().path()) {
        (_, "/") => serve_ui().await,
        ("GET", "/api/transforms") => list_transforms().await,
        ("POST", "/api/transform") => apply_transform(req).await,
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found\n".into())
            .map_err(Into::into),
    }
}

async fn serve_ui() -> anyhow::Result<Response<Body>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(UI_HTML.into())
        .map_err(Into::into)
}

async fn list_transforms() -> anyhow::Result<Response<Body>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(TRANSFORMS_JSON.into())
        .map_err(Into::into)
}

async fn apply_transform(mut req: Request<Body>) -> anyhow::Result<Response<Body>> {
    // Read the full binary body (binary-framed: [4 byte header len][JSON header][PNG data])
    let body_bytes = req.body_mut().contents().await?.to_vec();

    if body_bytes.len() < 4 {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("request too short".into())
            .map_err(Into::into);
    }

    // Forward the entire binary payload to the task-photon worker
    let timeout = Duration::from_secs(30).as_millis() as u32;

    match consumer::request("tasks.photon", &body_bytes, timeout) {
        Ok(resp) => {
            // Response is binary-framed: [4 byte header len][JSON header][PNG data]
            let resp_body = resp.body;

            if resp_body.len() < 4 {
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body("worker response too short".into())
                    .map_err(Into::into);
            }

            let header_len = u32::from_be_bytes([
                resp_body[0],
                resp_body[1],
                resp_body[2],
                resp_body[3],
            ]) as usize;

            if resp_body.len() < 4 + header_len {
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body("worker response truncated".into())
                    .map_err(Into::into);
            }

            let header_json = &resp_body[4..4 + header_len];
            let image_data = &resp_body[4 + header_len..];

            // Parse the header to get metadata
            let header_str = String::from_utf8_lossy(header_json);

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "image/png")
                .header("X-Processing-Info", header_str.as_ref())
                .body(image_data.to_vec().into())
                .map_err(Into::into)
        }
        Err(err) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(format!("worker error: {err}").into())
            .map_err(Into::into),
    }
}
