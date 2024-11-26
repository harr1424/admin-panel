use actix_web::{
    delete, get, patch, post,
    web::{Data, Json, Path},
    HttpResponse,
};
use chrono::NaiveDate;
use serde_json::json;
use std::{
    cmp::Ordering,
    collections::HashSet,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Language {
    English,
    Spanish,
    French,
    Italian,
    Portuguese,
    German,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum Status {
    Planning,
    Invited,
    Confirmed,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Planning => write!(f, "Planning"),
            Status::Invited => write!(f, "Invited"),
            Status::Confirmed => write!(f, "Confirmed"),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Engagement {
    pub id: Uuid,
    pub instructor: String,
    pub host: String,
    pub date: String,
    pub language: Language,
    pub title: String,
    pub part: usize,
    pub num_parts: usize,
    pub status: Status,
}

#[derive(Error, Debug)]
pub enum EngagementError {
    #[error("Failed to parse date '{0}': {1}")]
    DateParseError(String, #[source] chrono::ParseError),
    #[error("Invalid date format for engagement '{0}' with date '{1}'")]
    InvalidDateFormat(String, String),
}

impl Engagement {
    fn parse_date(&self) -> Result<NaiveDate, EngagementError> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|e| EngagementError::DateParseError(self.date.clone(), e))
    }

    fn validate(&self) -> Result<(), String> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(|_| {
            format!(
                "Invalid date format: {}. Expected format: YYYY-MM-DD",
                self.date
            )
        })?;

        if self.part == 0 {
            return Err("Part number must be greater than 0".to_string());
        }

        if self.num_parts == 0 {
            return Err("Number of parts must be greater than 0".to_string());
        }

        if self.part > self.num_parts {
            return Err(format!(
                "Part number ({}) cannot be greater than total number of parts ({})",
                self.part, self.num_parts
            ));
        }

        Ok(())
    }

    fn clean(&self) -> Self {

        Self {
            id: self.id, 
            instructor: ammonia::clean(&self.instructor),
            host: ammonia::clean(&self.host),
            date: ammonia::clean(&self.date),
            language: self.language.clone(),
            title: ammonia::clean(&self.title),
            part: self.part.clone(),
            num_parts: self.num_parts.clone(),
            status: self.status.clone(),
        }
    }
}

pub fn compare_engagement_dates(
    a: &Engagement,
    b: &Engagement,
) -> Result<Ordering, EngagementError> {
    match (a.parse_date(), b.parse_date()) {
        (Ok(date_a), Ok(date_b)) => Ok(date_a.cmp(&date_b)),
        (Err(_), _) => Err(EngagementError::InvalidDateFormat(
            a.instructor.clone(),
            a.date.clone(),
        )),
        (_, Err(_)) => Err(EngagementError::InvalidDateFormat(
            b.instructor.clone(),
            b.date.clone(),
        )),
    }
}

impl std::hash::Hash for Engagement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Engagement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Engagement {}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct NewEngagement {
    pub instructor: String,
    pub host: String,
    pub date: String,
    pub language: Language,
    pub title: String,
    pub part: usize,
    pub num_parts: usize,
    pub status: Status,
}

impl NewEngagement {
    fn validate(&self) -> Result<(), String> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(|_| {
            format!(
                "Invalid date format: {}. Expected format: YYYY-MM-DD",
                self.date
            )
        })?;

        if self.part == 0 {
            return Err("Part number must be greater than 0".to_string());
        }

        if self.num_parts == 0 {
            return Err("Number of parts must be greater than 0".to_string());
        }

        if self.part > self.num_parts {
            return Err(format!(
                "Part number ({}) cannot be greater than total number of parts ({})",
                self.part, self.num_parts
            ));
        }

        Ok(())
    }
}

#[post("/engs")]
pub async fn add_eng(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    body: Json<NewEngagement>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(validation_error) = body.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .json(json!({
                "error": "Validation failed",
                "details": validation_error
            })));
    }

    match repo.lock() {
        Ok(mut repo) => {
            let new_eng = Engagement {
                id: Uuid::new_v4(),
                instructor: ammonia::clean(&body.instructor),
                host: ammonia::clean(&body.host),
                date: ammonia::clean(&body.date),
                language: body.language.clone(),
                title: ammonia::clean(&body.title),
                part: body.part.clone(),
                num_parts: body.num_parts.clone(),
                status: body.status.clone(),
            };
            repo.insert(new_eng);

            Ok(HttpResponse::Created().finish())
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[get("engs/{lang}")]
pub async fn get_engs(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    lang: Path<Language>,
) -> Result<HttpResponse, actix_web::Error> {
    match repo.lock() {
        Ok(repo) => {
            let engagements: Vec<Engagement> = repo
                .iter()
                .filter(|x| x.language == *lang)
                .cloned()
                .collect();

            Ok(HttpResponse::Ok()
                .content_type("application/json; charset=utf-8")
                .json(engagements))
        }
        Err(_) => Err(actix_web::error::ErrorInternalServerError(
            "Failed to acquire repo lock",
        )),
    }
}

#[patch("/engs")]
pub async fn edit_eng(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    body: Json<Engagement>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(validation_error) = body.validate() {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .json(json!({
                "error": "Validation failed",
                "details": validation_error
            })));
    }

    match repo.lock() {
        Ok(mut repo) => {
            let target_eng = body.into_inner().clean();
            if repo.contains(&target_eng) {
                repo.remove(&target_eng);
                repo.insert(target_eng.clone());
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

#[delete("/engs")]
pub async fn delete_eng(
    repo: Data<Arc<Mutex<HashSet<Engagement>>>>,
    body: Json<Engagement>,
) -> Result<HttpResponse, actix_web::Error> {
    match repo.lock() {
        Ok(mut repo) => {
            let target_eng = body.into_inner();
            if repo.contains(&target_eng) {
                repo.remove(&target_eng);
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
