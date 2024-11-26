use aws_config::{meta::region::RegionProviderChain, BehaviorVersion, Region};
use aws_sdk_s3::error::SdkError as AwsSdkError;
use aws_sdk_s3::{primitives::ByteStream, Client as S3Client};
use chrono::{Duration, TimeZone, Utc};
use std::collections::HashSet;
use std::{
    error::Error as StdError,
    io::Cursor,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tokio::time::interval;

use crate::api::Engagement;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("AWS SDK error: {0}")]
    AwsError(#[from] aws_sdk_s3::Error),

    #[error("Compression error: {0}")]
    CompressionError(#[from] std::io::Error),

    #[error("Serialization Error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Environment variable error: {0}")]
    EnvError(#[from] std::env::VarError),

    #[error("AWS operation error: {0}")]
    AwsOperationError(String),

    #[error("Unknown error: {0}")]
    Unknown(#[from] Box<dyn StdError + Send + Sync>),
}

impl<E> From<AwsSdkError<E>> for BackupError
where
    E: std::fmt::Debug,
{
    fn from(err: AwsSdkError<E>) -> Self {
        BackupError::AwsOperationError(format!("{:?}", err))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupData {
    engagements: HashSet<Engagement>,
    instructors: HashSet<String>,
    hosts: HashSet<String>,
}

#[derive(Clone, Debug)]
pub struct BackupConfig {
    pub bucket_name: String,
    pub prefix: String,
    pub region: String,
    pub retention_days: i64,
    pub backup_interval_hours: u64,
    pub compression_level: i32,
}

impl BackupConfig {
    pub fn from_env() -> Result<Self, BackupError> {
        Ok(Self {
            bucket_name: std::env::var("AWS_BACKUP_BUCKET")?,
            prefix: std::env::var("AWS_BACKUP_PREFIX")
                .unwrap_or_else(|_| "message-backups".to_string()),
            region: std::env::var("AWS_REGION")?,
            retention_days: std::env::var("BACKUP_RETENTION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            backup_interval_hours: std::env::var("BACKUP_INTERVAL_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            compression_level: std::env::var("BACKUP_COMPRESSION_LEVEL")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
        })
    }
}

pub struct BackupMetrics {
    pub eng_count: usize,
    pub instructor_count: usize,
    pub host_count: usize,
    pub compressed_size: usize,
    pub compression_time_ms: u128,
    pub upload_time_ms: u128,
}

pub struct BackupSystem {
    engagements: Arc<Mutex<HashSet<Engagement>>>,
    instructors: Arc<Mutex<HashSet<String>>>,
    hosts: Arc<Mutex<HashSet<String>>>,
    config: BackupConfig,
    client: S3Client,
}

impl BackupSystem {
    pub async fn new(
        engagements: Arc<Mutex<HashSet<Engagement>>>,
        instructors: Arc<Mutex<HashSet<String>>>,
        hosts: Arc<Mutex<HashSet<String>>>,
        config: BackupConfig,
    ) -> Result<Self, BackupError> {
        let region = Region::new(config.region.clone());
        let region_provider = RegionProviderChain::first_try(region).or_default_provider();
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let client = S3Client::new(&sdk_config);

        Ok(Self {
            engagements,
            instructors,
            hosts,
            config,
            client,
        })
    }

    pub async fn start_backup_task(self) {
        let interval_secs = self.config.backup_interval_hours * 3600;
        let mut interval = interval(tokio::time::Duration::from_secs(interval_secs));

        log::info!(
            "Starting backup task with prefix {}, bucket {}, and interval {}",
            self.config.prefix,
            self.config.bucket_name,
            self.config.backup_interval_hours
        );

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                match self.perform_backup().await {
                    Ok(metrics) => {
                        log::info!(
                            "Backup task completed for {} messages, {} instructors, {} hosts, having compressed size {} bytes taking {} ms to compress, and uploaded in {} ms",
                            metrics.eng_count,
                            metrics.instructor_count, 
                            metrics.host_count,
                            metrics.compressed_size,
                            metrics.compression_time_ms,
                            metrics.upload_time_ms,
                        );
                    }
                    Err(e) => {
                        log::error!("Backup failed: {}", e);
                    }
                }
            }
        });
    }

    async fn perform_backup(&self) -> Result<BackupMetrics, BackupError> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let key = format!("{}/backup_{}.json.zst", self.config.prefix, timestamp);

        // Serialize engagements in a separate scope so the lock is dropped
        let (json, eng_count, instructor_count, host_count) = {
            let engagements = self.engagements.lock().unwrap();
            let instructors = self.instructors.lock().unwrap();
            let hosts = self.hosts.lock().unwrap();

            let backup_data = BackupData {
                engagements: engagements.clone(),
                instructors: instructors.clone(),
                hosts: hosts.clone(),
            };

            let json = serde_json::to_string(&backup_data)?;
            (json, engagements.len(), instructors.len(), hosts.len())
        };

        let compression_start = std::time::Instant::now();
        let compressed =
            zstd::stream::encode_all(Cursor::new(json.as_bytes()), self.config.compression_level)?;
        let compression_time = compression_start.elapsed();
        let compressed_size = compressed.len();

        let upload_start = std::time::Instant::now();
        self.client
            .put_object()
            .bucket(&self.config.bucket_name)
            .key(&key)
            .body(ByteStream::from(compressed))
            .content_type("application/zstd+bincode")
            .storage_class(aws_sdk_s3::types::StorageClass::StandardIa)
            .metadata("compressed_size", compressed_size.to_string())
            .metadata("message_count", eng_count.to_string())
            .metadata("instructor_count", instructor_count.to_string())
            .metadata("host_count", host_count.to_string())
            .send()
            .await
            .map_err(BackupError::from)?;
        let upload_time = upload_start.elapsed();

        self.cleanup_old_backups()
            .await
            .map_err(|e| BackupError::Unknown(e.to_string().into()))?;

        Ok(BackupMetrics {
            eng_count,
            instructor_count,
            host_count,
            compressed_size,
            compression_time_ms: compression_time.as_millis(),
            upload_time_ms: upload_time.as_millis(),
        })
    }

    async fn cleanup_old_backups(&self) -> Result<(), Box<dyn StdError>> {
        let cutoff_date = Utc::now() - Duration::days(self.config.retention_days);

        let objects = self
            .client
            .list_objects_v2()
            .bucket(&self.config.bucket_name)
            .prefix(&self.config.prefix)
            .send()
            .await?;

        for object in objects.contents() {
            if let (Some(key), Some(last_modified)) = (object.key(), object.last_modified()) {
                if let Ok(millis) = last_modified.to_millis() {
                    let last_modified = Utc.timestamp_millis_opt(millis).unwrap();

                    if last_modified < cutoff_date {
                        self.client
                            .delete_object()
                            .bucket(&self.config.bucket_name)
                            .key(key)
                            .send()
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn restore_latest_backup(
        &self,
    ) -> Result<(HashSet<Engagement>, HashSet<String>, HashSet<String>), Box<dyn std::error::Error>> {
        let objects = self
            .client
            .list_objects_v2()
            .bucket(&self.config.bucket_name)
            .prefix(&self.config.prefix)
            .send()
            .await?;

        let latest_key = objects
            .contents()
            .iter()
            .max_by_key(|obj| obj.last_modified())
            .and_then(|obj| obj.key())
            .ok_or("No backups found")?;

        let response = self
            .client
            .get_object()
            .bucket(&self.config.bucket_name)
            .key(latest_key)
            .send()
            .await?;

        let compressed_data = response.body.collect().await?.into_bytes();
        let decompressed = zstd::stream::decode_all(Cursor::new(compressed_data))?;
        let backup_data: BackupData = serde_json::from_slice(&decompressed)?;

        Ok((
            backup_data.engagements,
            backup_data.instructors,
            backup_data.hosts,
        ))
    }
}
