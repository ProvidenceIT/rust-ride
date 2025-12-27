//! WebSocket Streaming Server
//!
//! Handles WebSocket connections for real-time metrics streaming.

use super::{
    PinAuthenticator, StreamingConfig, StreamingError, StreamingEvent, StreamingMetrics,
    StreamingSession,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Trait for streaming server implementations
pub trait StreamingServer: Send + Sync {
    /// Start the streaming server
    fn start(
        &self,
        config: &StreamingConfig,
    ) -> impl std::future::Future<Output = Result<(), StreamingError>> + Send;

    /// Stop the server
    fn stop(&self) -> impl std::future::Future<Output = Result<(), StreamingError>> + Send;

    /// Check if server is running
    fn is_running(&self) -> bool;

    /// Get server URL
    fn get_url(&self) -> Option<String>;

    /// Generate QR code for connection URL
    fn get_qr_code(&self) -> Option<QrCodeData>;

    /// Regenerate PIN
    fn regenerate_pin(&self) -> String;

    /// Get current PIN
    fn get_pin(&self) -> Option<String>;

    /// Get connected sessions
    fn get_sessions(&self) -> Vec<StreamingSession>;

    /// Disconnect a session
    fn disconnect_session(
        &self,
        session_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), StreamingError>> + Send;

    /// Broadcast metrics update to all authenticated clients
    fn broadcast_metrics(&self, metrics: &StreamingMetrics);

    /// Subscribe to server events
    fn subscribe_events(&self) -> broadcast::Receiver<StreamingEvent>;
}

/// QR code data for easy connection
#[derive(Debug, Clone)]
pub struct QrCodeData {
    /// The URL encoded in the QR code
    pub url: String,
    /// QR code as ASCII art (for terminal display)
    pub ascii: String,
    /// QR code as SVG
    pub svg: String,
}

/// Default streaming server implementation
pub struct DefaultStreamingServer {
    config: Arc<RwLock<Option<StreamingConfig>>>,
    is_running: Arc<RwLock<bool>>,
    sessions: Arc<RwLock<HashMap<Uuid, StreamingSession>>>,
    pin_auth: Arc<dyn PinAuthenticator>,
    event_tx: broadcast::Sender<StreamingEvent>,
    metrics_tx: broadcast::Sender<StreamingMetrics>,
    server_url: Arc<RwLock<Option<String>>>,
}

impl DefaultStreamingServer {
    /// Create a new streaming server
    pub fn new(pin_auth: Arc<dyn PinAuthenticator>) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        let (metrics_tx, _) = broadcast::channel(100);

        Self {
            config: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pin_auth,
            event_tx,
            metrics_tx,
            server_url: Arc::new(RwLock::new(None)),
        }
    }

    /// Generate QR code for URL
    fn generate_qr_code(url: &str) -> QrCodeData {
        // TODO: Use qrcode crate to generate actual QR code
        // For now, return placeholder
        QrCodeData {
            url: url.to_string(),
            ascii: format!("[QR Code for {}]", url),
            svg: format!(r#"<svg><text>{}</text></svg>"#, url),
        }
    }

    /// Get local IP address for server URL
    fn get_local_ip() -> Option<String> {
        // Try to get local IP
        // This is a simplified implementation
        #[cfg(target_os = "windows")]
        {
            Some("127.0.0.1".to_string())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Some("127.0.0.1".to_string())
        }
    }
}

impl StreamingServer for DefaultStreamingServer {
    async fn start(&self, config: &StreamingConfig) -> Result<(), StreamingError> {
        if !config.enabled {
            return Err(StreamingError::BindFailed(
                "Streaming is disabled".to_string(),
            ));
        }

        *self.config.write().await = Some(config.clone());

        // Generate PIN if required
        if config.require_pin {
            self.pin_auth.generate_pin();
        }

        // Build server URL
        let ip = Self::get_local_ip().unwrap_or_else(|| "localhost".to_string());
        let url = format!("http://{}:{}", ip, config.port);
        *self.server_url.write().await = Some(url.clone());

        tracing::info!("Starting streaming server on port {}", config.port);

        // TODO: Actually start the WebSocket server using tokio-tungstenite
        // let addr = format!("0.0.0.0:{}", config.port);
        // let listener = TcpListener::bind(&addr).await
        //     .map_err(|e| StreamingError::BindFailed(e.to_string()))?;

        *self.is_running.write().await = true;

        let _ = self.event_tx.send(StreamingEvent::ServerStarted { url });

        Ok(())
    }

    async fn stop(&self) -> Result<(), StreamingError> {
        // Disconnect all sessions
        let sessions: Vec<Uuid> = self.sessions.read().await.keys().cloned().collect();
        for session_id in sessions {
            let _ = self.disconnect_session(&session_id).await;
        }

        *self.is_running.write().await = false;
        *self.server_url.write().await = None;

        let _ = self.event_tx.send(StreamingEvent::ServerStopped);

        tracing::info!("Streaming server stopped");

        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running.try_read().map(|r| *r).unwrap_or(false)
    }

    fn get_url(&self) -> Option<String> {
        self.server_url.try_read().ok()?.clone()
    }

    fn get_qr_code(&self) -> Option<QrCodeData> {
        let url = self.get_url()?;
        Some(Self::generate_qr_code(&url))
    }

    fn regenerate_pin(&self) -> String {
        self.pin_auth.regenerate()
    }

    fn get_pin(&self) -> Option<String> {
        self.pin_auth.get_current_pin()
    }

    fn get_sessions(&self) -> Vec<StreamingSession> {
        self.sessions
            .try_read()
            .map(|s| s.values().cloned().collect())
            .unwrap_or_default()
    }

    async fn disconnect_session(&self, session_id: &Uuid) -> Result<(), StreamingError> {
        let mut sessions = self.sessions.write().await;

        if sessions.remove(session_id).is_some() {
            let _ = self.event_tx.send(StreamingEvent::ClientDisconnected {
                session_id: *session_id,
            });
            Ok(())
        } else {
            Err(StreamingError::SessionNotFound(*session_id))
        }
    }

    fn broadcast_metrics(&self, metrics: &StreamingMetrics) {
        // Send to all connected clients
        let _ = self.metrics_tx.send(metrics.clone());
    }

    fn subscribe_events(&self) -> broadcast::Receiver<StreamingEvent> {
        self.event_tx.subscribe()
    }
}

/// Embedded HTML dashboard for external displays
pub const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RustRide Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1a1a2e;
            color: #eee;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }
        .auth-screen {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100vh;
        }
        .auth-screen input {
            font-size: 2rem;
            padding: 1rem;
            text-align: center;
            width: 200px;
            letter-spacing: 0.5rem;
        }
        .auth-screen button {
            margin-top: 1rem;
            padding: 1rem 2rem;
            font-size: 1.2rem;
            background: #4f46e5;
            color: white;
            border: none;
            border-radius: 0.5rem;
            cursor: pointer;
        }
        .dashboard { display: none; padding: 1rem; }
        .metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; }
        .metric {
            background: #16213e;
            padding: 1.5rem;
            border-radius: 1rem;
            text-align: center;
        }
        .metric-value { font-size: 3rem; font-weight: bold; color: #4f46e5; }
        .metric-label { font-size: 0.9rem; color: #888; margin-top: 0.5rem; }
        .connected { display: block; }
    </style>
</head>
<body>
    <div class="auth-screen" id="auth">
        <h1>RustRide</h1>
        <p>Enter PIN to connect</p>
        <input type="text" id="pin" maxlength="6" pattern="[0-9]*" inputmode="numeric">
        <button onclick="authenticate()">Connect</button>
    </div>
    <div class="dashboard" id="dashboard">
        <div class="metrics">
            <div class="metric">
                <div class="metric-value" id="power">--</div>
                <div class="metric-label">POWER (W)</div>
            </div>
            <div class="metric">
                <div class="metric-value" id="hr">--</div>
                <div class="metric-label">HEART RATE</div>
            </div>
            <div class="metric">
                <div class="metric-value" id="cadence">--</div>
                <div class="metric-label">CADENCE</div>
            </div>
            <div class="metric">
                <div class="metric-value" id="speed">--</div>
                <div class="metric-label">SPEED (km/h)</div>
            </div>
        </div>
    </div>
    <script>
        let ws;
        function authenticate() {
            const pin = document.getElementById('pin').value;
            ws = new WebSocket(`ws://${location.host}/ws`);
            ws.onopen = () => ws.send(JSON.stringify({type: 'auth', pin: pin}));
            ws.onmessage = (e) => {
                const data = JSON.parse(e.data);
                if (data.type === 'auth_ok') {
                    document.getElementById('auth').style.display = 'none';
                    document.getElementById('dashboard').style.display = 'block';
                } else if (data.type === 'metrics') {
                    if (data.power) document.getElementById('power').textContent = data.power;
                    if (data.heart_rate) document.getElementById('hr').textContent = data.heart_rate;
                    if (data.cadence) document.getElementById('cadence').textContent = data.cadence;
                    if (data.speed) document.getElementById('speed').textContent = data.speed.toFixed(1);
                }
            };
        }
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::streaming::pin::DefaultPinAuthenticator;

    #[test]
    fn test_server_creation() {
        let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
        let server = DefaultStreamingServer::new(pin_auth);

        assert!(!server.is_running());
        assert!(server.get_url().is_none());
    }

    #[tokio::test]
    async fn test_start_disabled() {
        let pin_auth = Arc::new(DefaultPinAuthenticator::new(60));
        let server = DefaultStreamingServer::new(pin_auth);

        let config = StreamingConfig {
            enabled: false,
            ..Default::default()
        };

        let result = server.start(&config).await;
        assert!(result.is_err());
    }
}
