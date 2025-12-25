//! Cloud ML API client with offline queue support.
//!
//! T010: Create MlClient for cloud API calls
//! T011: Implement offline queue
//! T012: Implement exponential backoff retry

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam::channel::{bounded, Receiver, Sender};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::Mutex;

use super::types::MlError;

/// Default cloud API base URL.
const DEFAULT_API_URL: &str = "https://api.rustride.io/v1";

/// Maximum queue size for offline requests.
const MAX_QUEUE_SIZE: usize = 50;

/// Retry intervals in seconds for exponential backoff.
const RETRY_INTERVALS: &[u64] = &[30, 60, 120, 300];

/// Cloud ML API client.
pub struct MlClient {
    /// HTTP client
    http: reqwest::Client,
    /// Base URL for API
    base_url: String,
    /// API key for authentication
    api_key: String,
    /// Whether the client is currently online
    online: Arc<AtomicBool>,
    /// Offline request queue
    offline_queue: Arc<Mutex<VecDeque<QueuedRequest>>>,
    /// Channel to signal queue processing
    queue_signal: (Sender<()>, Receiver<()>),
}

/// A queued request for offline processing.
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    /// API endpoint
    pub endpoint: String,
    /// Request body as JSON
    pub body_json: String,
    /// Number of retry attempts
    pub retry_count: u32,
    /// When the request was queued
    pub queued_at: std::time::Instant,
}

impl MlClient {
    /// Create a new ML client.
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, DEFAULT_API_URL.to_string())
    }

    /// Create a new ML client with custom base URL.
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http,
            base_url,
            api_key,
            online: Arc::new(AtomicBool::new(true)),
            offline_queue: Arc::new(Mutex::new(VecDeque::new())),
            queue_signal: bounded(1),
        }
    }

    /// Check if the client is currently online.
    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }

    /// Set online status.
    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::Relaxed);
    }

    /// Send a request to the cloud API.
    ///
    /// Returns the response or queues the request if offline.
    pub async fn request<T, R>(&self, endpoint: &str, body: &T) -> Result<R, MlError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        let body_json = serde_json::to_string(body)?;

        // Try the request
        match self.send_request::<R>(&url, &body_json).await {
            Ok(response) => {
                self.set_online(true);
                Ok(response)
            }
            Err(MlError::Offline) => {
                // Queue the request for later
                self.queue_request(endpoint.to_string(), body_json).await?;
                Err(MlError::Offline)
            }
            Err(e) => Err(e),
        }
    }

    /// Send HTTP request to the API.
    async fn send_request<R: DeserializeOwned>(
        &self,
        url: &str,
        body_json: &str,
    ) -> Result<R, MlError> {
        let response = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body_json.to_string())
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    self.set_online(false);
                    MlError::Offline
                } else {
                    MlError::ApiError(e.to_string())
                }
            })?;

        let status = response.status();

        if status.is_success() {
            let api_response: ApiResponse<R> = response
                .json()
                .await
                .map_err(|e| MlError::SerializationError(e.to_string()))?;

            if api_response.success {
                api_response.data.ok_or_else(|| {
                    MlError::ApiError("API returned success but no data".to_string())
                })
            } else {
                let error = api_response.error.unwrap_or_default();
                Err(MlError::ApiError(error.message))
            }
        } else if status.as_u16() == 429 {
            Err(MlError::RateLimited)
        } else if status.as_u16() == 422 {
            let api_response: ApiResponse<()> = response
                .json()
                .await
                .map_err(|e| MlError::SerializationError(e.to_string()))?;
            let error = api_response.error.unwrap_or_default();
            Err(MlError::InsufficientData {
                message: error.message,
                guidance: "Record more varied workouts to improve prediction accuracy.".to_string(),
            })
        } else if status.is_server_error() {
            self.set_online(false);
            Err(MlError::Offline)
        } else {
            Err(MlError::ApiError(format!(
                "API returned status {}",
                status
            )))
        }
    }

    /// Queue a request for later processing.
    async fn queue_request(&self, endpoint: String, body_json: String) -> Result<(), MlError> {
        let mut queue = self.offline_queue.lock().await;

        if queue.len() >= MAX_QUEUE_SIZE {
            // Remove oldest request to make room
            queue.pop_front();
        }

        queue.push_back(QueuedRequest {
            endpoint,
            body_json,
            retry_count: 0,
            queued_at: std::time::Instant::now(),
        });

        // Signal that there's work to do
        let _ = self.queue_signal.0.try_send(());

        Ok(())
    }

    /// Process queued requests when connectivity is restored.
    ///
    /// Returns the number of successfully processed requests.
    pub async fn flush_queue(&self) -> Result<usize, MlError> {
        if !self.is_online() {
            return Ok(0);
        }

        let mut processed = 0;
        let mut queue = self.offline_queue.lock().await;

        while let Some(mut request) = queue.pop_front() {
            let url = format!("{}{}", self.base_url, request.endpoint);

            // Get retry delay based on attempt count
            let retry_delay = RETRY_INTERVALS
                .get(request.retry_count as usize)
                .copied()
                .unwrap_or(300);

            // Try to send the request
            let result = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .body(request.body_json.clone())
                .send()
                .await;

            match result {
                Ok(response) if response.status().is_success() => {
                    processed += 1;
                }
                Ok(response) if response.status().as_u16() == 429 => {
                    // Rate limited - put back in queue with increased retry count
                    request.retry_count += 1;
                    if request.retry_count < RETRY_INTERVALS.len() as u32 {
                        queue.push_back(request);
                    }
                    // Wait before continuing
                    tokio::time::sleep(Duration::from_secs(retry_delay)).await;
                }
                Ok(_) => {
                    // Other error - discard request
                    tracing::warn!("Discarding queued request to {}: API error", request.endpoint);
                }
                Err(e) => {
                    // Network error - put back in queue
                    self.set_online(false);
                    request.retry_count += 1;
                    if request.retry_count < RETRY_INTERVALS.len() as u32 {
                        queue.push_front(request);
                    }
                    tracing::warn!("Failed to flush queue: {}", e);
                    break;
                }
            }
        }

        Ok(processed)
    }

    /// Get the number of queued requests.
    pub async fn queue_size(&self) -> usize {
        self.offline_queue.lock().await.len()
    }

    /// Check API health.
    pub async fn health_check(&self) -> Result<HealthStatus, MlError> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                self.set_online(false);
                if e.is_connect() || e.is_timeout() {
                    MlError::Offline
                } else {
                    MlError::ApiError(e.to_string())
                }
            })?;

        if response.status().is_success() {
            self.set_online(true);
            response
                .json()
                .await
                .map_err(|e| MlError::SerializationError(e.to_string()))
        } else {
            Err(MlError::ApiError(format!(
                "Health check failed: {}",
                response.status()
            )))
        }
    }
}

/// API response wrapper.
#[derive(Debug, serde::Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<ApiError>,
}

/// API error details.
#[derive(Debug, Default, serde::Deserialize)]
#[allow(dead_code)]
struct ApiError {
    code: String,
    message: String,
}

/// Health check response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub model_versions: ModelVersions,
}

/// Model version information.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelVersions {
    pub ftp_model: String,
    pub fatigue_model: String,
    pub recommendation_model: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MlClient::new("test-api-key".to_string());
        assert!(client.is_online());
    }

    #[test]
    fn test_online_status_toggle() {
        let client = MlClient::new("test-api-key".to_string());
        assert!(client.is_online());

        client.set_online(false);
        assert!(!client.is_online());

        client.set_online(true);
        assert!(client.is_online());
    }

    #[tokio::test]
    async fn test_queue_request() {
        let client = MlClient::new("test-api-key".to_string());
        client.set_online(false);

        // Queue should be empty initially
        assert_eq!(client.queue_size().await, 0);

        // Queue a request
        client
            .queue_request(
                "/predictions/ftp".to_string(),
                r#"{"user_id": "test"}"#.to_string(),
            )
            .await
            .unwrap();

        assert_eq!(client.queue_size().await, 1);
    }

    #[tokio::test]
    async fn test_queue_max_size() {
        let client = MlClient::new("test-api-key".to_string());

        // Fill the queue beyond max size
        for i in 0..MAX_QUEUE_SIZE + 5 {
            client
                .queue_request(format!("/test/{}", i), "{}".to_string())
                .await
                .unwrap();
        }

        // Queue should be capped at MAX_QUEUE_SIZE
        assert_eq!(client.queue_size().await, MAX_QUEUE_SIZE);
    }
}
