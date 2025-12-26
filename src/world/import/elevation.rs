//! Elevation service client for fetching missing elevation data.

use super::{GpsPoint, ImportError};

/// Elevation service configuration
pub struct ElevationService {
    client: reqwest::Client,
    base_url: String,
    batch_size: usize,
}

impl ElevationService {
    /// Create a new elevation service client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.open-elevation.com/api/v1/lookup".to_string(),
            batch_size: 100, // API limit per request
        }
    }

    /// Create with custom base URL (for testing or self-hosted)
    pub fn with_url(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            batch_size: 100,
        }
    }

    /// Fetch elevation data for points missing it
    ///
    /// Returns the number of points that had elevation fetched
    pub async fn fetch_elevation(&self, points: &mut [GpsPoint]) -> Result<u32, ImportError> {
        // Collect indices of points missing elevation
        let missing_indices: Vec<usize> = points
            .iter()
            .enumerate()
            .filter(|(_, p)| p.elevation.is_none())
            .map(|(i, _)| i)
            .collect();

        if missing_indices.is_empty() {
            return Ok(0);
        }

        let mut fetched_count = 0;

        // Process in batches
        for batch_indices in missing_indices.chunks(self.batch_size) {
            let locations: Vec<Location> = batch_indices
                .iter()
                .map(|&i| Location {
                    latitude: points[i].latitude,
                    longitude: points[i].longitude,
                })
                .collect();

            match self.fetch_batch(&locations).await {
                Ok(elevations) => {
                    for (i, elevation) in batch_indices.iter().zip(elevations.iter()) {
                        points[*i].elevation = Some(*elevation);
                        fetched_count += 1;
                    }
                }
                Err(e) => {
                    // Log error but continue - elevation is optional
                    tracing::warn!("Failed to fetch elevation batch: {}", e);
                }
            }
        }

        Ok(fetched_count)
    }

    /// Fetch elevation for a batch of locations
    async fn fetch_batch(&self, locations: &[Location]) -> Result<Vec<f32>, ImportError> {
        let request = ElevationRequest {
            locations: locations.to_vec(),
        };

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ImportError::ElevationFetchFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ImportError::ElevationFetchFailed(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        let result: ElevationResponse = response
            .json()
            .await
            .map_err(|e| ImportError::ElevationFetchFailed(e.to_string()))?;

        Ok(result
            .results
            .into_iter()
            .map(|r| r.elevation as f32)
            .collect())
    }
}

impl Default for ElevationService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct Location {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, serde::Serialize)]
struct ElevationRequest {
    locations: Vec<Location>,
}

#[derive(Debug, serde::Deserialize)]
struct ElevationResponse {
    results: Vec<ElevationResult>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct ElevationResult {
    latitude: f64,
    longitude: f64,
    elevation: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_service_creation() {
        let service = ElevationService::new();
        assert!(service.base_url.contains("open-elevation"));
    }

    #[test]
    fn test_custom_url() {
        let service = ElevationService::with_url("http://localhost:8080/api");
        assert_eq!(service.base_url, "http://localhost:8080/api");
    }
}
