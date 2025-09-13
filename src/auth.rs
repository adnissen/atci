// atci (andrew's transcript and clipping interface)
// Copyright (C) 2025 Andrew Nissen

use base64::{Engine, engine::general_purpose};
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};

pub struct AuthGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = crate::config::load_config_or_default();

        // If password is null/None, allow all requests through
        let password = match config.password.as_deref() {
            Some(p) => p,
            None => return Outcome::Success(AuthGuard),
        };

        // Check cookie first
        if let Some(cookie) = request.cookies().get("auth_token")
            && cookie.value() == password
        {
            return Outcome::Success(AuthGuard);
        }

        // Check basic auth
        if let Some(auth_header) = request.headers().get_one("Authorization")
            && let Some(basic_auth) = auth_header.strip_prefix("Basic ")
            && let Ok(decoded) = general_purpose::STANDARD.decode(basic_auth)
            && let Ok(credentials) = String::from_utf8(decoded)
            && let Some((_username, auth_password)) = credentials.split_once(':')
            && auth_password == password
        {
            return Outcome::Success(AuthGuard);
        }

        Outcome::Error((Status::Unauthorized, ()))
    }
}
