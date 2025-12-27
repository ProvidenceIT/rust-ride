//! PIN-based Authentication
//!
//! Generates and validates PINs for streaming authentication.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Trait for PIN authentication
pub trait PinAuthenticator: Send + Sync {
    /// Generate a new 6-digit PIN
    fn generate_pin(&self) -> String;

    /// Validate a PIN attempt
    fn validate_pin(&self, attempt: &str) -> bool;

    /// Get time until PIN expires
    fn time_until_expiry(&self) -> Duration;

    /// Check if PIN has expired
    fn is_expired(&self) -> bool;

    /// Regenerate PIN (invalidates old one)
    fn regenerate(&self) -> String;

    /// Get current PIN (for display)
    fn get_current_pin(&self) -> Option<String>;
}

/// Default PIN authenticator implementation
pub struct DefaultPinAuthenticator {
    current_pin: Mutex<Option<String>>,
    created_at: Mutex<Option<Instant>>,
    expiry_duration: Duration,
    failed_attempts: Mutex<u32>,
    max_failed_attempts: u32,
}

impl DefaultPinAuthenticator {
    /// Create a new PIN authenticator
    pub fn new(expiry_minutes: u32) -> Self {
        Self {
            current_pin: Mutex::new(None),
            created_at: Mutex::new(None),
            expiry_duration: Duration::from_secs(expiry_minutes as u64 * 60),
            failed_attempts: Mutex::new(0),
            max_failed_attempts: 5,
        }
    }

    /// Generate a random 6-digit PIN
    fn generate_random_pin() -> String {
        use std::time::SystemTime;

        // Simple random generation based on system time
        // In production, use a proper random source
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        // Generate 6 digits
        let pin = (seed % 1_000_000) as u32;
        format!("{:06}", pin)
    }
}

impl PinAuthenticator for DefaultPinAuthenticator {
    fn generate_pin(&self) -> String {
        let pin = Self::generate_random_pin();

        *self.current_pin.lock().unwrap() = Some(pin.clone());
        *self.created_at.lock().unwrap() = Some(Instant::now());
        *self.failed_attempts.lock().unwrap() = 0;

        tracing::info!("Generated new streaming PIN");

        pin
    }

    fn validate_pin(&self, attempt: &str) -> bool {
        // Check if locked out
        let failed = *self.failed_attempts.lock().unwrap();
        if failed >= self.max_failed_attempts {
            tracing::warn!("PIN locked due to too many failed attempts");
            return false;
        }

        // Check if expired
        if self.is_expired() {
            tracing::debug!("PIN has expired");
            return false;
        }

        // Check PIN
        let current = self.current_pin.lock().unwrap();
        let valid = current.as_ref().map(|p| p == attempt).unwrap_or(false);

        if valid {
            *self.failed_attempts.lock().unwrap() = 0;
            tracing::info!("PIN validated successfully");
        } else {
            *self.failed_attempts.lock().unwrap() += 1;
            tracing::warn!("Invalid PIN attempt");
        }

        valid
    }

    fn time_until_expiry(&self) -> Duration {
        let created = self.created_at.lock().unwrap();

        match *created {
            Some(created_at) => {
                let elapsed = created_at.elapsed();
                if elapsed >= self.expiry_duration {
                    Duration::ZERO
                } else {
                    self.expiry_duration - elapsed
                }
            }
            None => Duration::ZERO,
        }
    }

    fn is_expired(&self) -> bool {
        let created = self.created_at.lock().unwrap();

        match *created {
            Some(created_at) => created_at.elapsed() >= self.expiry_duration,
            None => true, // No PIN generated yet
        }
    }

    fn regenerate(&self) -> String {
        self.generate_pin()
    }

    fn get_current_pin(&self) -> Option<String> {
        if self.is_expired() {
            None
        } else {
            self.current_pin.lock().unwrap().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_generation() {
        let auth = DefaultPinAuthenticator::new(60);
        let pin = auth.generate_pin();

        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_pin_validation() {
        let auth = DefaultPinAuthenticator::new(60);
        let pin = auth.generate_pin();

        assert!(auth.validate_pin(&pin));
        assert!(!auth.validate_pin("000000"));
    }

    #[test]
    fn test_pin_expiry() {
        let auth = DefaultPinAuthenticator::new(0); // Immediate expiry
        let _pin = auth.generate_pin();

        // Wait a tiny bit
        std::thread::sleep(Duration::from_millis(10));

        assert!(auth.is_expired());
        assert!(auth.get_current_pin().is_none());
    }

    #[test]
    fn test_failed_attempts_lockout() {
        let auth = DefaultPinAuthenticator::new(60);
        let _pin = auth.generate_pin();

        // Make multiple failed attempts
        for _ in 0..5 {
            assert!(!auth.validate_pin("wrong1"));
        }

        // Should be locked out now
        assert!(!auth.validate_pin("wrong2"));
    }
}
