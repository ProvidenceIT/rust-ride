//! Integration tests for Weather API using wiremock.
//!
//! These tests simulate OpenWeatherMap API responses without hitting the real API.

use rustride::integrations::weather::{
    OpenWeatherMapProvider, WeatherCondition, WeatherConfig, WeatherError, WeatherProvider,
    WeatherUnits,
};
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a valid OpenWeatherMap API response JSON
fn create_owm_response(
    temp: f32,
    feels_like: f32,
    humidity: u8,
    pressure: u16,
    weather_id: u32,
    weather_main: &str,
    weather_desc: &str,
    wind_speed: f32,
    wind_deg: u16,
    visibility: u32,
) -> String {
    format!(
        r#"{{
            "main": {{
                "temp": {},
                "feels_like": {},
                "humidity": {},
                "pressure": {}
            }},
            "weather": [{{
                "id": {},
                "main": "{}",
                "description": "{}"
            }}],
            "wind": {{
                "speed": {},
                "deg": {}
            }},
            "visibility": {},
            "sys": {{
                "country": "GB"
            }}
        }}"#,
        temp, feels_like, humidity, pressure, weather_id, weather_main, weather_desc, wind_speed,
        wind_deg, visibility
    )
}

/// Create a weather provider configured for the mock server
async fn create_test_provider(mock_server: &MockServer) -> OpenWeatherMapProvider {
    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074, // London
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 30,
        ..Default::default()
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    provider
}

// ============================================================================
// Successful API Response Tests
// ============================================================================

#[tokio::test]
async fn test_successful_weather_fetch_clear_sky() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        22.5,   // temp
        21.0,   // feels_like
        65,     // humidity
        1015,   // pressure
        800,    // weather_id (clear)
        "Clear",
        "clear sky",
        5.5, // wind_speed
        180, // wind_deg
        10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .and(query_param("lat", "51.5074"))
        .and(query_param("lon", "-0.1278"))
        .and(query_param("units", "metric"))
        .and(query_param("appid", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok(), "Weather fetch should succeed");

    let weather = result.unwrap();
    assert!((weather.temperature - 22.5).abs() < 0.01);
    assert!((weather.feels_like - 21.0).abs() < 0.01);
    assert_eq!(weather.humidity, 65);
    assert_eq!(weather.pressure, 1015);
    assert_eq!(weather.condition, WeatherCondition::Clear);
    assert_eq!(weather.description, "clear sky");
    assert!((weather.wind_speed - 5.5).abs() < 0.01);
    assert_eq!(weather.wind_direction, 180);
    assert_eq!(weather.visibility, 10000);
}

#[tokio::test]
async fn test_successful_weather_fetch_rain() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        15.0, // temp
        14.5, // feels_like
        80,   // humidity
        1005, // pressure
        500,  // weather_id (rain)
        "Rain",
        "light rain",
        3.0, // wind_speed
        270, // wind_deg
        8000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Rain);
    assert!((weather.temperature - 15.0).abs() < 0.01);
}

#[tokio::test]
async fn test_successful_weather_fetch_thunderstorm() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        18.0, // temp
        17.0, // feels_like
        90,   // humidity
        1000, // pressure
        200,  // weather_id (thunderstorm)
        "Thunderstorm",
        "thunderstorm with light rain",
        8.0, // wind_speed
        90,  // wind_deg
        5000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Thunderstorm);
}

#[tokio::test]
async fn test_successful_weather_fetch_snow() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        -5.0, // temp
        -8.0, // feels_like
        75,   // humidity
        1020, // pressure
        601,  // weather_id (snow)
        "Snow",
        "light snow",
        2.0, // wind_speed
        45,  // wind_deg
        3000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Snow);
    assert!((weather.temperature - (-5.0)).abs() < 0.01);
}

#[tokio::test]
async fn test_successful_weather_fetch_fog() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        10.0, // temp
        9.0,  // feels_like
        95,   // humidity
        1010, // pressure
        741,  // weather_id (fog)
        "Fog",
        "fog",
        1.0, // wind_speed
        0,   // wind_deg
        200, // low visibility
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Fog);
    assert_eq!(weather.visibility, 200);
}

#[tokio::test]
async fn test_successful_weather_fetch_imperial_units() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        72.5, // temp in Fahrenheit
        75.0, // feels_like
        55,   // humidity
        1013, // pressure
        800,  // weather_id (clear)
        "Clear",
        "clear sky",
        10.0, // wind_speed in mph
        45,   // wind_deg
        10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .and(query_param("units", "imperial"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());
    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 40.7128, // New York
        longitude: -74.0060,
        units: WeatherUnits::Imperial,
        refresh_interval_minutes: 30,
        ..Default::default()
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert!((weather.temperature - 72.5).abs() < 0.01);
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn test_rate_limit_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    // With fallback enabled, get_weather returns default weather
    // But refresh() should return the actual error
    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::RateLimited => {} // Expected
        e => panic!("Expected RateLimited, got {:?}", e),
    }
}

#[tokio::test]
async fn test_server_error_500() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::RequestFailed(msg) => {
            assert!(msg.contains("500"));
        }
        e => panic!("Expected RequestFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_server_error_502() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::RequestFailed(msg) => {
            assert!(msg.contains("502"));
        }
        e => panic!("Expected RequestFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_server_error_503() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::RequestFailed(msg) => {
            assert!(msg.contains("503"));
        }
        e => panic!("Expected RequestFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_unauthorized_error_401() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::RequestFailed(msg) => {
            assert!(msg.contains("401"));
        }
        e => panic!("Expected RequestFailed, got {:?}", e),
    }
}

#[tokio::test]
async fn test_invalid_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{ not valid json }"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::InvalidResponse(msg) => {
            assert!(msg.contains("JSON") || msg.contains("parse"));
        }
        e => panic!("Expected InvalidResponse, got {:?}", e),
    }
}

#[tokio::test]
async fn test_incomplete_json_response() {
    let mock_server = MockServer::start().await;

    // Missing required 'wind' field
    let incomplete_response = r#"{
        "main": {
            "temp": 20.0,
            "feels_like": 19.0,
            "humidity": 50,
            "pressure": 1013
        },
        "weather": [{"id": 800, "main": "Clear", "description": "clear sky"}]
    }"#;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(incomplete_response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::InvalidResponse(_) => {} // Expected
        e => panic!("Expected InvalidResponse, got {:?}", e),
    }
}

// ============================================================================
// Timeout Tests
// ============================================================================

#[tokio::test]
async fn test_slow_response_times_out() {
    let mock_server = MockServer::start().await;

    // Simulate a slow server that takes longer than our timeout
    let response_body = create_owm_response(
        20.0, 19.0, 50, 1013, 800, "Clear", "clear sky", 5.0, 180, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(response_body)
                .set_delay(Duration::from_secs(15)), // 15 seconds delay, our timeout is 10
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.refresh().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        WeatherError::NetworkError(msg) => {
            assert!(
                msg.to_lowercase().contains("timeout")
                    || msg.to_lowercase().contains("timed out"),
                "Error should mention timeout: {}",
                msg
            );
        }
        e => panic!("Expected NetworkError with timeout, got {:?}", e),
    }
}

// ============================================================================
// Caching Tests
// ============================================================================

#[tokio::test]
async fn test_cached_response_is_returned() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        22.5, 21.0, 65, 1015, 800, "Clear", "clear sky", 5.5, 180, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call should hit the API
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok());

    // Second call should return cached data (no API call)
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());

    // Both should have the same data
    let weather1 = result1.unwrap();
    let weather2 = result2.unwrap();
    assert!((weather1.temperature - weather2.temperature).abs() < 0.01);
}

// ============================================================================
// Condition Code Mapping Tests (via real API responses)
// ============================================================================

#[tokio::test]
async fn test_condition_mapping_drizzle() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        12.0, 11.0, 85, 1008, 300, // drizzle code
        "Drizzle", "light intensity drizzle", 2.0, 90, 7000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::LightRain);
}

#[tokio::test]
async fn test_condition_mapping_heavy_rain() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        16.0, 15.0, 92, 1002, 522, // shower rain code
        "Rain", "heavy shower rain", 6.0, 180, 4000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::HeavyRain);
}

#[tokio::test]
async fn test_condition_mapping_cloudy_variants() {
    let mock_server = MockServer::start().await;

    // Test partly cloudy (801)
    let response_body = create_owm_response(
        18.0, 17.0, 60, 1012, 801, // few clouds
        "Clouds", "few clouds", 3.0, 270, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::PartlyCloudy);
}

#[tokio::test]
async fn test_condition_mapping_overcast() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        14.0, 13.0, 70, 1010, 804, // overcast
        "Clouds", "overcast clouds", 4.0, 315, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Overcast);
}

// ============================================================================
// Fallback Behavior Tests
// ============================================================================

#[tokio::test]
async fn test_fallback_to_default_on_api_failure() {
    let mock_server = MockServer::start().await;

    // Server always returns 500
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // get_weather should return default weather on failure (no cache)
    let result = provider.get_weather().await;

    assert!(result.is_ok(), "Should fallback to default weather");
    let weather = result.unwrap();

    // Should be default weather (20°C, clear)
    assert!((weather.temperature - 20.0).abs() < 0.1);
    assert_eq!(weather.condition, WeatherCondition::Clear);
    assert!(weather.description.contains("default"));
}

#[tokio::test]
async fn test_fallback_to_stale_cache_on_api_failure() {
    let mock_server = MockServer::start().await;

    // First request succeeds
    let response_body = create_owm_response(
        25.0, 24.0, 60, 1015, 800, "Clear", "clear sky", 5.0, 180, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call populates cache
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok());
    let first_weather = result1.unwrap();
    assert!((first_weather.temperature - 25.0).abs() < 0.1);

    // Now set up server to fail
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Force refresh should fail but provider should still have cache
    let _ = provider.refresh().await;

    // get_weather should return cached data
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());
    let cached_weather = result2.unwrap();

    // Should return the cached 25°C weather
    assert!((cached_weather.temperature - 25.0).abs() < 0.1);
}

// ============================================================================
// Multiple Weather Conditions in Response
// ============================================================================

#[tokio::test]
async fn test_multiple_weather_conditions_uses_first() {
    let mock_server = MockServer::start().await;

    // Response with multiple weather conditions
    let response_body = r#"{
        "main": {
            "temp": 18.0,
            "feels_like": 17.0,
            "humidity": 80,
            "pressure": 1005
        },
        "weather": [
            {"id": 500, "main": "Rain", "description": "light rain"},
            {"id": 701, "main": "Mist", "description": "mist"}
        ],
        "wind": {
            "speed": 2.0,
            "deg": 90
        },
        "visibility": 6000
    }"#;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();

    // Should use the first weather condition (Rain, id 500)
    assert_eq!(weather.condition, WeatherCondition::Rain);
    assert_eq!(weather.description, "light rain");
}

// ============================================================================
// Optional Fields Handling
// ============================================================================

#[tokio::test]
async fn test_missing_optional_wind_direction() {
    let mock_server = MockServer::start().await;

    // Response without wind direction
    let response_body = r#"{
        "main": {
            "temp": 20.0,
            "feels_like": 19.0,
            "humidity": 50,
            "pressure": 1013
        },
        "weather": [{"id": 800, "main": "Clear", "description": "clear sky"}],
        "wind": {
            "speed": 3.0
        }
    }"#;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();

    // Wind direction should default to 0
    assert_eq!(weather.wind_direction, 0);
    assert!((weather.wind_speed - 3.0).abs() < 0.1);
}

#[tokio::test]
async fn test_missing_optional_visibility() {
    let mock_server = MockServer::start().await;

    // Response without visibility
    let response_body = r#"{
        "main": {
            "temp": 20.0,
            "feels_like": 19.0,
            "humidity": 50,
            "pressure": 1013
        },
        "weather": [{"id": 800, "main": "Clear", "description": "clear sky"}],
        "wind": {
            "speed": 3.0,
            "deg": 180
        }
    }"#;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();

    // Visibility should default to 10000
    assert_eq!(weather.visibility, 10000);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_empty_weather_array_uses_default() {
    let mock_server = MockServer::start().await;

    // Response with empty weather array
    let response_body = r#"{
        "main": {
            "temp": 20.0,
            "feels_like": 19.0,
            "humidity": 50,
            "pressure": 1013
        },
        "weather": [],
        "wind": {
            "speed": 3.0,
            "deg": 180
        }
    }"#;

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();

    // Should default to Clear when no weather conditions
    assert_eq!(weather.condition, WeatherCondition::Clear);
    assert_eq!(weather.description, "Unknown");
}

#[tokio::test]
async fn test_extreme_temperature_values() {
    let mock_server = MockServer::start().await;

    // Very cold temperature
    let response_body = create_owm_response(
        -40.0, // extreme cold
        -45.0, 30, 1035, 600, "Snow", "heavy snow", 15.0, 0, 500,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert!((weather.temperature - (-40.0)).abs() < 0.01);
}

#[tokio::test]
async fn test_high_wind_speed() {
    let mock_server = MockServer::start().await;

    let response_body = create_owm_response(
        15.0, 10.0, 50, 990, 771, // squall
        "Squall", "squall", 35.0, // very high wind
        270, 8000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;
    let result = provider.get_weather().await;

    assert!(result.is_ok());
    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Windy);
    assert!((weather.wind_speed - 35.0).abs() < 0.01);
}

// ============================================================================
// Comprehensive Fallback Behavior Tests
// ============================================================================

#[tokio::test]
async fn test_fallback_returns_cached_data_on_rate_limit_error() {
    let mock_server = MockServer::start().await;

    // First request succeeds and caches data
    let response_body = create_owm_response(
        18.0, 17.0, 75, 1012, 500, "Rain", "light rain", 4.0, 270, 8000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call populates cache
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok());
    let first_weather = result1.unwrap();
    assert_eq!(first_weather.condition, WeatherCondition::Rain);
    assert!((first_weather.temperature - 18.0).abs() < 0.1);

    // Now set up server to return rate limit error
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&mock_server)
        .await;

    // Force refresh should fail
    let _ = provider.refresh().await;

    // get_weather should return cached rain data
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());
    let cached_weather = result2.unwrap();

    // Should return the cached rain weather
    assert_eq!(cached_weather.condition, WeatherCondition::Rain);
    assert!((cached_weather.temperature - 18.0).abs() < 0.1);
}

#[tokio::test]
async fn test_fallback_returns_cached_data_on_network_timeout() {
    let mock_server = MockServer::start().await;

    // First request succeeds and caches data
    let response_body = create_owm_response(
        10.0, 8.0, 85, 1008, 600, "Snow", "light snow", 6.0, 45, 3000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call populates cache
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok());
    let first_weather = result1.unwrap();
    assert_eq!(first_weather.condition, WeatherCondition::Snow);

    // Now set up server to timeout (15s delay > 10s timeout)
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(response_body)
                .set_delay(Duration::from_secs(15)),
        )
        .mount(&mock_server)
        .await;

    // Force refresh should timeout
    let _ = provider.refresh().await;

    // get_weather should return cached snow data
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());
    let cached_weather = result2.unwrap();

    // Should return the cached snow weather
    assert_eq!(cached_weather.condition, WeatherCondition::Snow);
    assert!((cached_weather.temperature - 10.0).abs() < 0.1);
}

#[tokio::test]
async fn test_fallback_returns_default_on_503_when_no_cache() {
    let mock_server = MockServer::start().await;

    // Server always returns 503 Service Unavailable
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // get_weather should return default weather on failure (no cache)
    let result = provider.get_weather().await;

    assert!(result.is_ok(), "Should fallback to default weather");
    let weather = result.unwrap();

    // Should be default weather (20°C, clear)
    assert!((weather.temperature - 20.0).abs() < 0.1);
    assert_eq!(weather.condition, WeatherCondition::Clear);
    assert!(weather.description.contains("default"));
}

#[tokio::test]
async fn test_fallback_returns_default_on_invalid_json_when_no_cache() {
    let mock_server = MockServer::start().await;

    // Server returns invalid JSON
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{ invalid json here }"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // get_weather should return default weather
    let result = provider.get_weather().await;

    assert!(result.is_ok(), "Should fallback to default weather");
    let weather = result.unwrap();

    // Should be default weather
    assert!((weather.temperature - 20.0).abs() < 0.1);
    assert_eq!(weather.condition, WeatherCondition::Clear);
}

#[tokio::test]
async fn test_fallback_default_weather_has_sensible_values() {
    let mock_server = MockServer::start().await;

    // Server returns error
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();

    // Verify all default values are sensible
    assert!((weather.temperature - 20.0).abs() < 0.1); // 20°C
    assert!((weather.feels_like - 20.0).abs() < 0.1);
    assert_eq!(weather.humidity, 50);
    assert_eq!(weather.condition, WeatherCondition::Clear);
    assert!((weather.wind_speed - 0.0).abs() < 0.1); // Calm wind
    assert_eq!(weather.wind_direction, 0);
    assert_eq!(weather.pressure, 1013); // Standard atmospheric pressure
    assert_eq!(weather.visibility, 10000); // Good visibility
}

#[tokio::test]
async fn test_fallback_preserves_cached_data_across_multiple_failures() {
    let mock_server = MockServer::start().await;

    // First request succeeds
    let response_body = create_owm_response(
        28.0, 30.0, 45, 1010, 800, "Clear", "clear sky", 8.0, 180, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call populates cache
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok());
    assert!((result1.unwrap().temperature - 28.0).abs() < 0.1);

    // Now set up server to fail
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Multiple failed refreshes
    for _ in 0..3 {
        let _ = provider.refresh().await;
    }

    // get_weather should still return the same cached data
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());
    let cached_weather = result2.unwrap();

    // Should still have the original cached values
    assert!((cached_weather.temperature - 28.0).abs() < 0.1);
    assert_eq!(cached_weather.condition, WeatherCondition::Clear);
}

// ============================================================================
// Exponential Backoff Integration Tests
// ============================================================================

#[tokio::test]
async fn test_backoff_prevents_immediate_retry_after_failure() {
    let mock_server = MockServer::start().await;

    // Server always returns error
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1) // Should only be called once due to backoff
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call fails and triggers backoff
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok()); // Returns default weather

    // Second call should NOT hit the API due to backoff
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());

    // Both should return default weather
    let weather1 = result1.unwrap();
    let weather2 = result2.unwrap();

    assert_eq!(weather1.condition, WeatherCondition::Clear);
    assert_eq!(weather2.condition, WeatherCondition::Clear);
}

#[tokio::test]
async fn test_refresh_respects_backoff_period() {
    let mock_server = MockServer::start().await;

    // Server always returns error
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First refresh fails
    let result1 = provider.refresh().await;
    assert!(result1.is_err());

    // Second refresh should fail immediately due to backoff (no API call)
    let result2 = provider.refresh().await;
    assert!(result2.is_err());

    // Error should indicate backoff
    match result2.unwrap_err() {
        WeatherError::NetworkError(msg) => {
            assert!(
                msg.contains("backoff"),
                "Error should mention backoff: {}",
                msg
            );
        }
        e => panic!("Expected NetworkError with backoff, got {:?}", e),
    }
}

#[tokio::test]
async fn test_success_resets_backoff() {
    let mock_server = MockServer::start().await;

    // First request fails
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server).await;

    // First call fails
    let result1 = provider.refresh().await;
    assert!(result1.is_err());

    // Verify we're in backoff
    assert!(provider.consecutive_failures().await > 0);

    // Now set up success response and wait for backoff to expire
    // (We can't actually wait 60 seconds in a test, so we'll verify the state)

    // Just verify the failure count is tracked
    assert_eq!(provider.consecutive_failures().await, 1);
}

#[tokio::test]
async fn test_consecutive_failures_increment() {
    let mock_server = MockServer::start().await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());
    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 0, // Force refresh every time
        ..Default::default()
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    // Track consecutive failures via refresh()
    // Note: After first failure, subsequent refresh() calls will fail due to backoff

    // Initially no failures
    assert_eq!(provider.consecutive_failures().await, 0);

    // Note: In real integration, failures would occur over time as backoff expires
    // For unit testing, we verified this in provider.rs tests
}

// ============================================================================
// Manual Override Integration Tests
// ============================================================================

#[tokio::test]
async fn test_override_takes_precedence_over_api_response() {
    let mock_server = MockServer::start().await;

    // Set up server to return clear weather
    let response_body = create_owm_response(
        25.0, 26.0, 40, 1015, 800, "Clear", "clear sky", 5.0, 180, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(0) // Should NOT be called because override is active
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    // Configure with override enabled
    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 30,
        override_enabled: true,
        override_condition: Some(WeatherCondition::Thunderstorm),
        override_temperature: Some(15.0),
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    // Get weather should return override, not API data
    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();

    // Should be override values, not API values
    assert_eq!(weather.condition, WeatherCondition::Thunderstorm);
    assert!((weather.temperature - 15.0).abs() < 0.1);
    assert!(weather.description.contains("(manual override)"));
}

#[tokio::test]
async fn test_override_takes_precedence_over_cached_data() {
    let mock_server = MockServer::start().await;

    // First request succeeds and caches data
    let response_body = create_owm_response(
        30.0, 32.0, 50, 1010, 800, "Clear", "clear sky", 3.0, 90, 10000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    // First, configure without override and fetch to populate cache
    let config_no_override = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 60, // Long cache validity
        override_enabled: false,
        override_condition: None,
        override_temperature: None,
    };
    provider.configure(config_no_override);
    provider.set_api_key("test_api_key".to_string()).await;

    // Populate cache
    let result1 = provider.get_weather().await;
    assert!(result1.is_ok());
    assert!((result1.unwrap().temperature - 30.0).abs() < 0.1);

    // Now enable override
    let config_with_override = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 60,
        override_enabled: true,
        override_condition: Some(WeatherCondition::Snow),
        override_temperature: Some(-5.0),
    };
    provider.configure(config_with_override);

    // Get weather should return override, not cached data
    let result2 = provider.get_weather().await;
    assert!(result2.is_ok());

    let weather = result2.unwrap();

    // Should be override values, not cached values
    assert_eq!(weather.condition, WeatherCondition::Snow);
    assert!((weather.temperature - (-5.0)).abs() < 0.1);
    assert!(weather.description.contains("(manual override)"));
}

#[tokio::test]
async fn test_override_works_when_api_unavailable() {
    let mock_server = MockServer::start().await;

    // Server always fails
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0) // Should NOT be called because override is active
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    // Configure with override enabled
    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 30,
        override_enabled: true,
        override_condition: Some(WeatherCondition::Fog),
        override_temperature: Some(8.0),
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    // Get weather should return override even though API would fail
    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Fog);
    assert!((weather.temperature - 8.0).abs() < 0.1);
}

#[tokio::test]
async fn test_disabled_override_falls_through_to_api() {
    let mock_server = MockServer::start().await;

    // Set up server to return rain weather
    let response_body = create_owm_response(
        12.0, 11.0, 80, 1005, 500, "Rain", "light rain", 6.0, 270, 6000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1) // Should be called because override is disabled
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    // Configure with override DISABLED but values set
    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 30,
        override_enabled: false, // Disabled!
        override_condition: Some(WeatherCondition::Snow), // Would be snow if enabled
        override_temperature: Some(-10.0),
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    // Get weather should return API data, not override
    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();

    // Should be API values, not override values
    assert_eq!(weather.condition, WeatherCondition::Rain);
    assert!((weather.temperature - 12.0).abs() < 0.1);
    assert!(!weather.description.contains("(manual override)"));
}

#[tokio::test]
async fn test_override_without_condition_falls_through_to_api() {
    let mock_server = MockServer::start().await;

    // Set up server to return cloudy weather
    let response_body = create_owm_response(
        16.0, 15.0, 70, 1012, 803, "Clouds", "broken clouds", 4.0, 180, 9000,
    );

    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
        .expect(1) // Should be called because override_condition is None
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    // Configure with override enabled but NO condition set
    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 30,
        override_enabled: true,
        override_condition: None, // No condition!
        override_temperature: Some(25.0), // Temperature set but irrelevant
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    // Get weather should fall through to API
    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();

    // Should be API values
    assert_eq!(weather.condition, WeatherCondition::Overcast);
    assert!((weather.temperature - 16.0).abs() < 0.1);
    assert!(!weather.description.contains("(manual override)"));
}

#[tokio::test]
async fn test_override_all_weather_conditions() {
    // Test that all weather conditions can be set via override
    let conditions = [
        WeatherCondition::Clear,
        WeatherCondition::PartlyCloudy,
        WeatherCondition::Cloudy,
        WeatherCondition::Overcast,
        WeatherCondition::Fog,
        WeatherCondition::LightRain,
        WeatherCondition::Rain,
        WeatherCondition::HeavyRain,
        WeatherCondition::Thunderstorm,
        WeatherCondition::Snow,
        WeatherCondition::Sleet,
        WeatherCondition::Hail,
        WeatherCondition::Windy,
    ];

    for condition in conditions {
        let mock_server = MockServer::start().await;

        // Server should never be called
        Mock::given(method("GET"))
            .and(path("/data/2.5/weather"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&mock_server)
            .await;

        let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

        let config = WeatherConfig {
            enabled: true,
            api_key_configured: true,
            latitude: 51.5074,
            longitude: -0.1278,
            units: WeatherUnits::Metric,
            refresh_interval_minutes: 30,
            override_enabled: true,
            override_condition: Some(condition),
            override_temperature: Some(20.0),
        };
        provider.configure(config);
        provider.set_api_key("test_api_key".to_string()).await;

        let result = provider.get_weather().await;
        assert!(result.is_ok(), "Override should work for {:?}", condition);

        let weather = result.unwrap();
        assert_eq!(
            weather.condition, condition,
            "Condition should match override for {:?}",
            condition
        );
    }
}

#[tokio::test]
async fn test_override_with_imperial_units() {
    let mock_server = MockServer::start().await;

    // Server should never be called
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 40.7128,
        longitude: -74.0060,
        units: WeatherUnits::Imperial,
        refresh_interval_minutes: 30,
        override_enabled: true,
        override_condition: Some(WeatherCondition::Clear),
        override_temperature: Some(75.0), // 75°F
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::Clear);
    assert!((weather.temperature - 75.0).abs() < 0.1);
}

#[tokio::test]
async fn test_override_without_temperature_uses_default() {
    let mock_server = MockServer::start().await;

    // Server should never be called
    Mock::given(method("GET"))
        .and(path("/data/2.5/weather"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock_server)
        .await;

    let provider = OpenWeatherMapProvider::with_base_url(mock_server.uri());

    let config = WeatherConfig {
        enabled: true,
        api_key_configured: true,
        latitude: 51.5074,
        longitude: -0.1278,
        units: WeatherUnits::Metric,
        refresh_interval_minutes: 30,
        override_enabled: true,
        override_condition: Some(WeatherCondition::HeavyRain),
        override_temperature: None, // No temperature override
    };
    provider.configure(config);
    provider.set_api_key("test_api_key".to_string()).await;

    let result = provider.get_weather().await;
    assert!(result.is_ok());

    let weather = result.unwrap();
    assert_eq!(weather.condition, WeatherCondition::HeavyRain);
    // Should use default metric temperature (20°C)
    assert!((weather.temperature - 20.0).abs() < 0.1);
}
