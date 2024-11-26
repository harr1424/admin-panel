
use actix_web::{
    delete, get, post,
    web::{Data, Path},
    HttpResponse,
};

use crate::types::HostRepo;

#[post("/hosts/{new}")]
pub async fn add_host(
    repo: Data<HostRepo>,
    new: Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let sanitized = ammonia::clean(&new);

    match repo.lock() {
        Ok(mut repo) => {
            repo.insert(sanitized);
            Ok(HttpResponse::Created().finish())
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[get("/hosts")]
pub async fn get_hosts(repo: Data<HostRepo>) -> Result<HttpResponse, actix_web::Error> {
    match repo.lock() {
        Ok(repo) => {
            let hosts: Vec<String> = repo.iter().cloned().collect();

            Ok(HttpResponse::Ok()
                .content_type("application/json; charset=utf-8")
                .json(hosts))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[delete("/hosts/{h}")]
pub async fn delete_host(
    repo: Data<HostRepo>,
    h: Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let sanitized = ammonia::clean(&h);

    match repo.lock() {
        Ok(mut repo) => {
            if repo.contains(&sanitized) {
                repo.remove(&sanitized);
                Ok(HttpResponse::Ok().finish())
            } else {
                Ok(HttpResponse::NotFound().finish())
            }
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}
