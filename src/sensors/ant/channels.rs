//! ANT+ Channel Management
//!
//! Handles allocation and management of ANT+ radio channels.

use super::{AntDeviceType, AntError, AntEvent};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Status of an ANT+ channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    /// Channel is unassigned
    Unassigned,
    /// Channel is assigned but not open
    Assigned,
    /// Channel is searching for devices
    Searching,
    /// Channel is open and receiving data
    Open,
    /// Channel is closed
    Closed,
    /// Channel has an error
    Error(String),
}

/// Represents an ANT+ channel
#[derive(Debug, Clone)]
pub struct AntChannel {
    /// Channel number (0-7 typically)
    pub number: u8,
    /// Current status
    pub status: ChannelStatus,
    /// Device type this channel is searching for/connected to
    pub device_type: Option<AntDeviceType>,
    /// Device ID if connected
    pub device_id: Option<u16>,
    /// Transmission type
    pub transmission_type: u8,
    /// Channel period (in 32768 Hz ticks)
    pub period: u16,
    /// RF frequency offset from 2400 MHz
    pub rf_frequency: u8,
}

impl AntChannel {
    /// Create a new unassigned channel
    pub fn new(number: u8) -> Self {
        Self {
            number,
            status: ChannelStatus::Unassigned,
            device_type: None,
            device_id: None,
            transmission_type: 0,
            period: 8070,     // Default period
            rf_frequency: 57, // ANT+ frequency (2457 MHz)
        }
    }

    /// Check if channel is available for assignment
    pub fn is_available(&self) -> bool {
        matches!(
            self.status,
            ChannelStatus::Unassigned | ChannelStatus::Closed
        )
    }

    /// Check if channel is actively receiving data
    pub fn is_active(&self) -> bool {
        matches!(self.status, ChannelStatus::Open)
    }
}

/// Channel configuration for device search
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// Device type to search for
    pub device_type: AntDeviceType,
    /// Specific device ID (0 for wildcard search)
    pub device_id: u16,
    /// Transmission type (0 for wildcard)
    pub transmission_type: u8,
    /// Search timeout in seconds
    pub search_timeout: u32,
}

impl ChannelConfig {
    /// Create config for searching any device of given type
    pub fn search_any(device_type: AntDeviceType) -> Self {
        Self {
            device_type,
            device_id: 0,
            transmission_type: 0,
            search_timeout: 30,
        }
    }

    /// Create config for specific device
    pub fn specific(device_type: AntDeviceType, device_id: u16, transmission_type: u8) -> Self {
        Self {
            device_type,
            device_id,
            transmission_type,
            search_timeout: 10,
        }
    }
}

/// Trait for managing ANT+ channels
pub trait AntChannelManager: Send + Sync {
    /// Allocate a channel for device search
    fn allocate_channel(
        &self,
        config: ChannelConfig,
    ) -> impl std::future::Future<Output = Result<u8, AntError>> + Send;

    /// Start searching on a channel
    fn start_search(
        &self,
        channel: u8,
    ) -> impl std::future::Future<Output = Result<(), AntError>> + Send;

    /// Close a channel
    fn close_channel(
        &self,
        channel: u8,
    ) -> impl std::future::Future<Output = Result<(), AntError>> + Send;

    /// Get channel status
    fn get_channel_status(&self, channel: u8) -> Option<ChannelStatus>;

    /// Get all channels
    fn get_channels(&self) -> Vec<AntChannel>;

    /// Subscribe to channel events
    fn subscribe_events(&self) -> broadcast::Receiver<AntEvent>;
}

/// Default implementation of channel manager
pub struct DefaultChannelManager {
    channels: Arc<RwLock<Vec<AntChannel>>>,
    event_tx: broadcast::Sender<AntEvent>,
}

impl DefaultChannelManager {
    /// Create a new channel manager with specified number of channels
    pub fn new(num_channels: u8) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        let channels: Vec<AntChannel> = (0..num_channels).map(AntChannel::new).collect();

        Self {
            channels: Arc::new(RwLock::new(channels)),
            event_tx,
        }
    }
}

impl AntChannelManager for DefaultChannelManager {
    async fn allocate_channel(&self, config: ChannelConfig) -> Result<u8, AntError> {
        let mut channels = self.channels.write().await;

        // Find first available channel
        let channel = channels
            .iter_mut()
            .find(|c| c.is_available())
            .ok_or_else(|| {
                AntError::ChannelAllocationFailed("No available channels".to_string())
            })?;

        let channel_num = channel.number;

        // Configure the channel
        channel.status = ChannelStatus::Assigned;
        channel.device_type = Some(config.device_type);
        channel.device_id = if config.device_id == 0 {
            None
        } else {
            Some(config.device_id)
        };
        channel.transmission_type = config.transmission_type;

        // Set period based on device type
        channel.period = match config.device_type {
            AntDeviceType::HeartRate => 8070,        // ~4.06 Hz
            AntDeviceType::Power => 8182,            // ~4.00 Hz
            AntDeviceType::SpeedCadence => 8086,     // ~4.05 Hz
            AntDeviceType::FitnessEquipment => 8192, // ~4.00 Hz
            AntDeviceType::Unknown(_) => 8070,
        };

        tracing::info!(
            "Allocated channel {} for {:?}",
            channel_num,
            config.device_type
        );

        Ok(channel_num)
    }

    async fn start_search(&self, channel_num: u8) -> Result<(), AntError> {
        let mut channels = self.channels.write().await;

        let channel = channels
            .iter_mut()
            .find(|c| c.number == channel_num)
            .ok_or(AntError::ChannelAllocationFailed(format!(
                "Channel {} not found",
                channel_num
            )))?;

        if !matches!(channel.status, ChannelStatus::Assigned) {
            return Err(AntError::ChannelAllocationFailed(format!(
                "Channel {} not assigned",
                channel_num
            )));
        }

        // TODO: Send actual ANT+ commands to start search
        channel.status = ChannelStatus::Searching;

        tracing::info!("Started search on channel {}", channel_num);

        Ok(())
    }

    async fn close_channel(&self, channel_num: u8) -> Result<(), AntError> {
        let mut channels = self.channels.write().await;

        let channel = channels
            .iter_mut()
            .find(|c| c.number == channel_num)
            .ok_or(AntError::ChannelAllocationFailed(format!(
                "Channel {} not found",
                channel_num
            )))?;

        // TODO: Send actual ANT+ close channel command
        channel.status = ChannelStatus::Closed;
        channel.device_type = None;
        channel.device_id = None;

        tracing::info!("Closed channel {}", channel_num);

        Ok(())
    }

    fn get_channel_status(&self, channel_num: u8) -> Option<ChannelStatus> {
        if let Ok(channels) = self.channels.try_read() {
            channels
                .iter()
                .find(|c| c.number == channel_num)
                .map(|c| c.status.clone())
        } else {
            None
        }
    }

    fn get_channels(&self) -> Vec<AntChannel> {
        if let Ok(channels) = self.channels.try_read() {
            channels.clone()
        } else {
            Vec::new()
        }
    }

    fn subscribe_events(&self) -> broadcast::Receiver<AntEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let channel = AntChannel::new(0);
        assert_eq!(channel.number, 0);
        assert!(channel.is_available());
        assert!(!channel.is_active());
    }

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::search_any(AntDeviceType::Power);
        assert_eq!(config.device_id, 0);
        assert_eq!(config.transmission_type, 0);

        let specific = ChannelConfig::specific(AntDeviceType::HeartRate, 12345, 1);
        assert_eq!(specific.device_id, 12345);
    }

    #[tokio::test]
    async fn test_channel_manager() {
        let manager = DefaultChannelManager::new(8);
        let channels = manager.get_channels();
        assert_eq!(channels.len(), 8);
        assert!(channels.iter().all(|c| c.is_available()));
    }
}
