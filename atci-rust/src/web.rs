use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{get, routes, response::content};
use rocket::response::status::NotFound;
use std::sync::Arc;
use crate::config::AtciConfig;
use crate::{files, queue, search, transcripts, Asset};

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

#[get("/app")]
fn app() -> Result<content::RawHtml<std::borrow::Cow<'static, [u8]>>, NotFound<String>> {
    match Asset::get("index.html") {
        Some(content) => Ok(content::RawHtml(content.data)),
        None => Err(NotFound("index.html not found".to_string())),
    }
}

#[get("/assets/<file..>")]
fn assets(file: std::path::PathBuf) -> Result<(rocket::http::ContentType, std::borrow::Cow<'static, [u8]>), NotFound<String>> {
    let filename = file.to_string_lossy();
    match Asset::get(&filename) {
        Some(content) => {
            let content_type = match file.extension().and_then(|ext| ext.to_str()) {
                Some("html") => rocket::http::ContentType::HTML,
                Some("css") => rocket::http::ContentType::CSS,
                Some("js") => rocket::http::ContentType::JavaScript,
                Some("json") => rocket::http::ContentType::JSON,
                Some("png") => rocket::http::ContentType::PNG,
                Some("jpg") | Some("jpeg") => rocket::http::ContentType::JPEG,
                Some("gif") => rocket::http::ContentType::GIF,
                Some("svg") => rocket::http::ContentType::SVG,
                Some("ico") => rocket::http::ContentType::Icon,
                Some("otf") | Some("ttf") => rocket::http::ContentType::Binary,
                _ => rocket::http::ContentType::Binary,
            };
            Ok((content_type, content.data))
        }
        None => Err(NotFound(format!("Asset {} not found", filename))),
    }
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
            app,
            assets,
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