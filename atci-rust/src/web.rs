use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{get, routes, response::content};
use rocket::response::status::NotFound;
use rocket::response::Redirect;
use crate::{config, files, queue, search, transcripts, clipper, tools_manager, model_manager, Asset};

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
fn index() -> Redirect {
    Redirect::to("/app")
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


pub async fn launch_server(host: &str, port: u16) -> Result<(), rocket::Error> {
    let figment = rocket::Config::figment()
        .merge(("address", host))
        .merge(("port", port));

    rocket::custom(figment)
        .mount("/", routes![
            index,
            health,
            config::web_get_config,
            config::web_set_config,
            app,
            assets,
            files::web_get_files,
            files::web_get_sources,
            clipper::web_clip,
            queue::web_get_queue,
            queue::web_get_queue_status,
            queue::web_block_path,
            queue::web_set_queue,
            search::web_search_transcripts,
            transcripts::web_get_transcript_by_path,
            transcripts::web_replace_transcript,
            transcripts::web_regenerate_transcript,
            transcripts::web_rename_transcript,
            tools_manager::web_list_tools,
            tools_manager::web_download_tool,
            tools_manager::web_use_downloaded_tool,
            model_manager::web_list_models,
            model_manager::web_download_model
        ])
        .launch()
        .await?;

    Ok(())
}