use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::outcome::Outcome;
use base64::{Engine, engine::general_purpose};

pub struct AuthGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = crate::config::load_config_or_default();
        let password = config.nonlocal_password.as_deref().unwrap_or("default-password");
        
        // Check cookie first
        if let Some(cookie) = request.cookies().get("auth_token") {
            if cookie.value() == password {
                return Outcome::Success(AuthGuard);
            }
        }

        // Check basic auth
        if let Some(auth_header) = request.headers().get_one("Authorization") {
            if let Some(basic_auth) = auth_header.strip_prefix("Basic ") {
                if let Ok(decoded) = general_purpose::STANDARD.decode(basic_auth) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((_username, auth_password)) = credentials.split_once(':') {
                            if auth_password == password {
                                return Outcome::Success(AuthGuard);
                            }
                        }
                    }
                }
            }
        }

        Outcome::Error((Status::Unauthorized, ()))
    }
}