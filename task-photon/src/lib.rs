mod transforms;

wit_bindgen::generate!({
    path: "../wit",
    world: "task",
    with: {
        "wasmcloud:messaging/types@0.2.0": generate,
        "wasmcloud:messaging/consumer@0.2.0": generate,
    },
});

use crate::wasmcloud::messaging::types::BrokerMessage;
use wasmcloud::messaging::consumer;
#[allow(unused)]
use wstd::prelude::*;

use photon_rs::PhotonImage;

struct Component;
export!(Component);

impl exports::wasmcloud::messaging::handler::Guest for Component {
    fn handle_message(msg: BrokerMessage) -> Result<(), String> {
        let Some(subject) = msg.reply_to else {
            return Err("missing reply_to".to_string());
        };

        // Decode binary-framed message: [4 bytes header len][JSON header][PNG image data]
        let body = &msg.body;
        if body.len() < 4 {
            return Err("message too short".to_string());
        }

        let header_len =
            u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize;

        if body.len() < 4 + header_len {
            return Err("message truncated".to_string());
        }

        let header_json = &body[4..4 + header_len];
        let image_bytes = &body[4 + header_len..];

        let request: transforms::TransformRequest =
            serde_json::from_slice(header_json)
                .map_err(|e| format!("invalid request JSON: {e}"))?;

        // Decode PNG bytes into PhotonImage
        let img = PhotonImage::new_from_byteslice(image_bytes.to_vec());

        let start = std::time::Instant::now();

        // Apply the transform
        let result_img = transforms::apply_transform(img, &request.transform, &request.params)
            .map_err(|e| format!("transform failed: {e}"))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Encode result back to PNG bytes
        let result_bytes = result_img.get_bytes();
        let width = result_img.get_width();
        let height = result_img.get_height();

        // Build response: binary-framed [header_len][JSON][PNG]
        let resp_header = serde_json::json!({
            "width": width,
            "height": height,
            "processing_time_ms": elapsed_ms,
        });
        let resp_header_bytes = serde_json::to_vec(&resp_header)
            .map_err(|e| format!("failed to serialize response: {e}"))?;

        let mut response_body = Vec::with_capacity(4 + resp_header_bytes.len() + result_bytes.len());
        response_body.extend_from_slice(&(resp_header_bytes.len() as u32).to_be_bytes());
        response_body.extend_from_slice(&resp_header_bytes);
        response_body.extend_from_slice(&result_bytes);

        let reply = BrokerMessage {
            subject,
            body: response_body,
            reply_to: None,
        };

        consumer::publish(&reply)
    }
}
