use actix_identity::Identity;
use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    get, post,
    web::{Data, Form},
    Error, HttpMessage, HttpRequest, HttpResponse,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use askama::Template;
use serde::Deserialize;
use std::{collections::HashMap, sync::Mutex};

pub struct UserStore(pub Mutex<HashMap<String, String>>);

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}

// Login page
#[get("/auth/login")]
pub async fn login_page(user: Option<Identity>) -> Result<HttpResponse, Error> {
    if user.is_some() {
        return Ok(HttpResponse::Found()
            .insert_header(("Location", "/views/index"))
            .finish());
    }

    let template = LoginTemplate { error: None };
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(template.render().map_err(ErrorInternalServerError)?))
}

#[post("/auth/login")]
pub async fn login(
    users: Data<UserStore>,
    form: Form<LoginForm>,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let users = users.0.lock().unwrap();
    
    if let Some(stored_hash) = users.get(&form.username) {
        let parsed_hash = PasswordHash::new(stored_hash)
            .map_err(|_| ErrorUnauthorized("Invalid credentials"))?;
            
        if Argon2::default()
            .verify_password(form.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            Identity::login(&request.extensions(), form.username.clone())
                .map_err(|_| ErrorInternalServerError("Could not create identity"))?;
            return Ok(HttpResponse::Found()
                .insert_header(("Location", "/views/index"))
                .finish());
        }
    }
    
    let template = LoginTemplate {
        error: Some("Invalid username or password".to_string()),
    };
    
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(template.render().map_err(ErrorInternalServerError)?))
}

#[post("/auth/logout")]
pub async fn logout(user: Identity) -> Result<HttpResponse, Error> {
    user.logout();
    Ok(HttpResponse::Found()
        .insert_header(("Location", "/auth/login"))
        .finish())
}

pub fn create_user(users: &UserStore, username: &str, password: &str) -> Result<(), String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?
        .to_string();
    
    let mut users = users.0.lock().unwrap();
    users.insert(username.to_string(), password_hash);
    Ok(())
}