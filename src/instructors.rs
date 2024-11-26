
use crate::types::InstructorRepo;
use actix_web::{
    delete, get, post,
    web::{Data, Path},
    HttpResponse,
};

#[post("/instructors/{new}")]
pub async fn add_instructor(
    repo: Data<InstructorRepo>,
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

#[get("/instructors")]
pub async fn get_instructors(repo: Data<InstructorRepo>) -> Result<HttpResponse, actix_web::Error> {
    match repo.lock() {
        Ok(repo) => {
            let instructors: Vec<String> = repo.iter().cloned().collect();

            Ok(HttpResponse::Ok()
                .content_type("application/json; charset=utf-8")
                .json(instructors))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[delete("/instructors/{i}")]
pub async fn delete_instructor(
    repo: Data<InstructorRepo>,
    i: Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let sanitized = ammonia::clean(&i);

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
