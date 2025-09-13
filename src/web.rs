// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use crate::{
    Asset, auth::AuthGuard, clipper, config, files, model_manager, queue, search, tools_manager,
    transcripts,
};
use rocket::form::{Form, FromForm};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::response::Redirect;
use rocket::response::status::NotFound;
use rocket::serde::Serialize;
use rocket::serde::json::Json;
use rocket::{Request, catch, catchers, get, post, response::content, routes};
use rocket_dyn_templates::{Template, context};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(FromForm)]
struct AuthForm {
    password: String,
    redirect: Option<String>,
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
fn app(
    _auth: AuthGuard,
) -> Result<content::RawHtml<std::borrow::Cow<'static, [u8]>>, NotFound<String>> {
    match Asset::get("index.html") {
        Some(content) => Ok(content::RawHtml(content.data)),
        None => Err(NotFound("index.html not found".to_string())),
    }
}

#[get("/assets/<file..>")]
fn assets(
    file: std::path::PathBuf,
) -> Result<(rocket::http::ContentType, std::borrow::Cow<'static, [u8]>), NotFound<String>> {
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

#[get("/auth?<redirect>")]
fn auth_page(redirect: Option<String>) -> Template {
    Template::render(
        "auth",
        context! {
            redirect: redirect.unwrap_or_else(|| "/app".to_string()),
            error: None::<String>
        },
    )
}

#[post("/auth", data = "<form>")]
fn auth_submit(form: Form<AuthForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Template> {
    let config = config::load_config_or_default();
    let expected_password = config.password.as_deref().unwrap_or("default-password");

    if form.password == expected_password {
        // Set authentication cookie
        let cookie = Cookie::build(("auth_token", form.password.clone()))
            .same_site(SameSite::Lax)
            .http_only(true)
            .path("/")
            .build();
        cookies.add(cookie);

        // Redirect to intended destination
        let redirect_url = form.redirect.as_deref().unwrap_or("/app");
        Ok(Redirect::to(redirect_url.to_string()))
    } else {
        // Return auth page with error
        Err(Template::render(
            "auth",
            context! {
                redirect: form.redirect.as_deref().unwrap_or("/app"),
                error: "Invalid password"
            },
        ))
    }
}

#[get("/logout")]
fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove("auth_token");
    Redirect::to("/auth")
}

fn api_routes() -> Vec<rocket::Route> {
    routes![
        health,
        config::web_get_config,
        config::web_set_config,
        files::web_get_files,
        files::web_get_sources,
        clipper::web_clip,
        clipper::web_frame,
        queue::web_get_queue,
        queue::web_get_queue_status,
        queue::web_block_path,
        queue::web_set_queue,
        queue::web_cancel_queue,
        search::web_search_transcripts,
        transcripts::web_get_transcript_by_path,
        transcripts::web_replace_transcript,
        transcripts::web_regenerate_transcript,
        transcripts::web_rename_transcript,
        tools_manager::web_list_tools,
        tools_manager::web_download_tool,
        tools_manager::web_use_downloaded_tool,
        model_manager::web_list_models,
        model_manager::web_download_model,
        crate::video_processor::web_get_subtitle_streams
    ]
}

#[catch(401)]
fn unauthorized(req: &Request) -> Result<Redirect, Status> {
    // Check if this is a browser request (HTML accept header) vs API request
    let accept_header = req.headers().get_one("Accept").unwrap_or("");
    let is_browser_request = accept_header.contains("text/html");

    if is_browser_request {
        let redirect_url = format!(
            "/auth?redirect={}",
            urlencoding::encode(req.uri().path().as_str())
        );
        Ok(Redirect::to(redirect_url))
    } else {
        // For API requests, return 401 JSON
        Err(Status::Unauthorized)
    }
}

pub async fn launch_server(host: &str, port: u16) -> Result<(), rocket::Error> {
    let figment = rocket::Config::figment()
        .merge(("template_dir", "templates/"))
        .merge(("address", host))
        .merge(("port", port));

    let mut all_routes = routes![index, auth_page, auth_submit, logout, app, assets];
    all_routes.extend(api_routes());

    rocket::custom(figment)
        .mount("/", all_routes)
        .register("/", catchers![unauthorized])
        .attach(Template::fairing())
        .launch()
        .await?;

    Ok(())
}

pub async fn launch_api_server(host: &str, port: u16) -> Result<(), rocket::Error> {
    let figment = rocket::Config::figment()
        .merge(("template_dir", "templates/"))
        .merge(("address", host))
        .merge(("port", port));

    rocket::custom(figment)
        .mount("/", api_routes())
        .register("/", catchers![unauthorized])
        .launch()
        .await?;

    Ok(())
}
