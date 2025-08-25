use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{get, routes};
use std::sync::Arc;
use crate::config::AtciConfig;
use crate::{files, queue, search, transcripts};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[get("/")]
fn index() -> &'static str {
    "ATCI Web Server - Video Processing API"
}

#[get("/api/health")]
fn health() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::success("OK"))
}


pub async fn launch_server(host: &str, port: u16, config: AtciConfig) -> Result<(), rocket::Error> {
    let figment = rocket::Config::figment()
        .merge(("address", host))
        .merge(("port", port));

    rocket::custom(figment)
        .manage(Arc::new(config))
        .mount("/", routes![
            index,
            health,
            files::web_get_files,
            queue::web_get_queue,
            queue::web_get_queue_status,
            search::web_search_transcripts,
            transcripts::web_get_transcript
        ])
        .launch()
        .await?;

    Ok(())
}