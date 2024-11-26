use actix_identity::IdentityMiddleware;
use actix_route_rate_limiter::{LimiterBuilder, RateLimiter};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, middleware::Logger, web::{self, Data}, App, HttpServer};
use auth::UserStore;
use dotenv::dotenv;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
};

mod api;
mod auth;
mod auth_middleware;
mod backup;
mod hosts;
mod instructors;
mod routing;
mod security_headers;
mod types;
mod views;

use api::Engagement;
use backup::{BackupConfig, BackupSystem};
use security_headers::SecurityHeaders;
use types::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect("Failed to read .env file");
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let listen_addr = env::var("LISTEN_ADDR").expect("LISTEN_HTTP must be set");
    let cert_path = env::var("TLS_CERT_PATH").expect("TLS_CERT_PATH must be set");
    let key_path = env::var("TLS_KEY_PATH").expect("TLS_KEY_PATH must be set");
    let username = env::var("ADMIN_USERNAME").expect("ADMIN_USERNAME must be set");
    let password = env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set");
    let rustls_config = load_rustls_config(&cert_path, &key_path)?;
    let secret_key = Key::generate();

    let engagements: Arc<Mutex<HashSet<Engagement>>> = Arc::new(Mutex::new(HashSet::new()));
    let instructors = InstructorRepo::new();
    let hosts = HostRepo::new();

    let backup_engagements = engagements.clone();
    let backup_instructors = instructors.clone();
    let backup_hosts = hosts.clone();

    if let Err(e) =
        configure_backup_system(backup_engagements.clone(), backup_instructors, backup_hosts).await
    {
        log::error!("Failed to configure backup system: {}", e);
    }

    let limiter = LimiterBuilder::new()
        .with_duration(chrono::Duration::minutes(1))
        .with_num_requests(60)
        .build();

    let users = Data::new(UserStore(Mutex::new(HashMap::new())));
    auth::create_user(
        &users,
        &username,
        &password,
    )
    .expect("Failed to create admin user");

    HttpServer::new(move || {
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(Logger::default())
            .wrap(SecurityHeaders)
            .wrap(RateLimiter::new(Arc::clone(&limiter)))
            .app_data(Data::new(engagements.clone()))
            .app_data(Data::new(instructors.clone()))
            .app_data(Data::new(hosts.clone()))
            .app_data(users.clone())
            // Public routes (login)
            .service(auth::login_page)
            .service(auth::login)
            .service(auth::logout)
            // Protected routes
            .service(
                web::scope("")
                    .wrap(auth_middleware::AuthMiddleware)
                    .configure(routing::config_eng_paths)
                    .configure(routing::config_view_paths)
                    .configure(routing::config_ins_paths)
                    .configure(routing::config_hosts_paths)
            )
    })
    .bind_rustls(&listen_addr, rustls_config)?
    .run()
    .await
}

fn load_rustls_config(cert_path: &str, key_path: &str) -> std::io::Result<ServerConfig> {
    let cert_file = &mut BufReader::new(File::open(cert_path)?);
    let cert_chain = certs(cert_file)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid cert"))?
        .into_iter()
        .map(Certificate)
        .collect();

    let key_file = &mut BufReader::new(File::open(key_path)?);
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key"))?
        .into_iter()
        .map(PrivateKey)
        .collect();

    if keys.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "No private key found",
        ));
    }

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(config)
}

async fn configure_backup_system(
    engagements: Arc<Mutex<HashSet<Engagement>>>,
    instructors: InstructorRepo,
    hosts: HostRepo,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = BackupConfig::from_env()?;
    let backup_system = BackupSystem::new(
        engagements.clone(),
        instructors.0.clone(),
        hosts.0.clone(),
        config,
    )
    .await?;

    {
        let (mut engagements_guard, mut instructors_guard, mut hosts_guard) = (
            engagements.lock().unwrap(),
            instructors.lock().unwrap(),
            hosts.lock().unwrap(),
        );

        if engagements_guard.is_empty() || instructors_guard.is_empty() || hosts_guard.is_empty() {
            match backup_system.restore_latest_backup().await {
                Ok((restored_engagements, restored_instructors, restored_hosts)) => {
                    if engagements_guard.is_empty() {
                        *engagements_guard = restored_engagements;
                        log::info!("Successfully restored engagements from latest backup");
                    }
                    if instructors_guard.is_empty() {
                        *instructors_guard = restored_instructors;
                        log::info!("Successfully restored instructors from latest backup");
                    }
                    if hosts_guard.is_empty() {
                        *hosts_guard = restored_hosts;
                        log::info!("Successfully restored hosts from latest backup");
                    }
                }
                Err(e) => {
                    log::error!("Failed to restore data from backup: {}", e);
                }
            }
        }
    }

    backup_system.start_backup_task().await;

    Ok(())
}
