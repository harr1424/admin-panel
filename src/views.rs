use actix_identity::Identity;
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use askama_actix::Template;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use crate::api::{compare_engagement_dates, Engagement, Language};
use crate::types::*;

#[derive(Template)]
#[template(path = "index.html")]
struct EngagementTemplate {
    engagements: Vec<Engagement>,
    engagements_json: Vec<String>,
    lang: String,
    all_langs: Vec<String>,
    has_language: bool,
    unique_instructors: Vec<String>,
    unique_hosts: Vec<String>,
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewEngagementTemplate {
    lang: String,
    all_langs: Vec<String>,
    has_language: bool,
    instructors: Vec<String>,
    hosts: Vec<String>,
}

#[derive(Template)]
#[template(path = "manage.html")]
struct ManageTemplate {
    instructors: Vec<String>,
    hosts: Vec<String>,
}

#[get("/views/index")]
pub async fn index_root(_user: Identity) -> Result<HttpResponse, actix_web::Error> {
    let all_langs = vec![
        "English".to_string(),
        "Spanish".to_string(),
        "French".to_string(),
        "Italian".to_string(),
        "Portuguese".to_string(),
        "German".to_string(),
    ];

    let template = EngagementTemplate {
        engagements: Vec::new(),
        engagements_json: Vec::new(),
        lang: String::new(),
        all_langs,
        has_language: false,
        unique_instructors: Vec::new(),
        unique_hosts: Vec::new(),
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            template
                .render()
                .map_err(actix_web::error::ErrorInternalServerError)?,
        ))
}

#[get("/views/index/{lang}")]
pub async fn index(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    lang: Path<Language>,
    _user: Identity,
) -> Result<HttpResponse, actix_web::Error> {
    let all_langs = vec![
        "English".to_string(),
        "Spanish".to_string(),
        "French".to_string(),
        "Italian".to_string(),
        "Portuguese".to_string(),
        "German".to_string(),
    ];

    match repo.lock() {
        Ok(repo) => {
            let mut engagements: Vec<Engagement> = repo
                .iter()
                .filter(|x| x.language == *lang)
                .cloned()
                .collect();

            engagements.sort_by(|a, b| match compare_engagement_dates(a, b) {
                Ok(ordering) => ordering,
                Err(_) => {
                    log::error!(
                        "Failed to compare dates between engagements '{}' ({}) and '{}' ({})",
                        a.instructor,
                        a.date,
                        b.instructor,
                        b.date
                    );
                    a.date.cmp(&b.date)
                }
            });

            let engagements_json: Vec<String> = engagements
                .iter()
                .map(|eng| {
                    serde_json::to_string(eng)
                        .unwrap_or_default()
                        .replace('\'', "\\'")
                })
                .collect();

            let mut unique_instructors: Vec<String> = engagements
                .iter()
                .map(|e| e.instructor.clone())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect();
            unique_instructors.sort();

            let mut unique_hosts: Vec<String> = engagements
                .iter()
                .map(|e| e.host.clone())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect();
            unique_hosts.sort();

            let template = EngagementTemplate {
                engagements,
                engagements_json,
                lang: format!("{:?}", *lang),
                all_langs,
                has_language: true,
                unique_instructors,
                unique_hosts,
            };

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(
                    template
                        .render()
                        .map_err(actix_web::error::ErrorInternalServerError)?,
                ))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[get("/views/new")]
pub async fn new_engagement_root(_user: Identity) -> Result<HttpResponse, actix_web::Error> {
    let all_langs = vec![
        "English".to_string(),
        "Spanish".to_string(),
        "French".to_string(),
        "Italian".to_string(),
        "Portuguese".to_string(),
        "German".to_string(),
    ];

    let template = NewEngagementTemplate {
        lang: String::new(),
        all_langs,
        has_language: false,
        instructors: Vec::new(),
        hosts: Vec::new(),
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            template
                .render()
                .map_err(actix_web::error::ErrorInternalServerError)?,
        ))
}

#[get("/views/new/{lang}")]
pub async fn new_engagement(
    lang: Path<Language>,
    instructor_repo: Data<InstructorRepo>,
    host_repo: Data<HostRepo>,
    _user: Identity,
) -> Result<HttpResponse, actix_web::Error> {
    let all_langs = vec![
        "English".to_string(),
        "Spanish".to_string(),
        "French".to_string(),
        "Italian".to_string(),
        "Portuguese".to_string(),
        "German".to_string(),
    ];

    let instructors = match instructor_repo.lock() {
        Ok(repo) => {
            let mut instructors: Vec<String> = repo.iter().cloned().collect();
            instructors.sort();
            instructors
        }
        Err(_) => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to acquire instructor repo lock",
            ))
        }
    };

    let hosts = match host_repo.lock() {
        Ok(repo) => {
            let mut hosts: Vec<String> = repo.iter().cloned().collect();
            hosts.sort();
            hosts
        }
        Err(_) => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to acquire host repo lock",
            ))
        }
    };

    let template = NewEngagementTemplate {
        lang: format!("{:?}", *lang),
        all_langs,
        has_language: true,
        instructors,
        hosts,
    };

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            template
                .render()
                .map_err(actix_web::error::ErrorInternalServerError)?,
        ))
}

#[get("/views/manage")]
pub async fn manage(
    instructor_repo: Data<InstructorRepo>,
    host_repo: Data<HostRepo>,
    _user: Identity,
) -> Result<HttpResponse, actix_web::Error> {
    let instructors = match instructor_repo.lock() {
        Ok(repo) => {
            let mut instructors: Vec<String> = repo.iter().cloned().collect();
            instructors.sort();
            println!("Instructors: {:?}", instructors);
            instructors
        }
        Err(_) => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to acquire instructor repo lock",
            ))
        }
    };

    // Get hosts from host repo
    let hosts = match host_repo.lock() {
        Ok(repo) => {
            let mut hosts: Vec<String> = repo.iter().cloned().collect();
            hosts.sort();
            println!("Hosts: {:?}", hosts);
            hosts
        }
        Err(_) => {
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to acquire host repo lock",
            ))
        }
    };

    let template = ManageTemplate { instructors, hosts };

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            template
                .render()
                .map_err(actix_web::error::ErrorInternalServerError)?,
        ))
}
