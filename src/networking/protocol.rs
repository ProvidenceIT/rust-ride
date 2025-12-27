//! Network protocol definitions for LAN multiplayer.
//!
//! Defines message types for UDP communication between peers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol version for compatibility checking.
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum message size in bytes (UDP safe).
pub const MAX_MESSAGE_SIZE: usize = 1400;

/// Protocol message types for LAN communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Session announcement (host broadcasts availability)
    SessionAnnounce {
        session_id: Uuid,
        host_rider_id: Uuid,
        host_name: String,
        session_name: Option<String>,
        world_id: String,
        participant_count: u8,
        max_participants: u8,
    },

    /// Join request from a peer
    JoinRequest {
        session_id: Uuid,
        rider_id: Uuid,
        rider_name: String,
    },

    /// Join accepted response
    JoinAccepted {
        session_id: Uuid,
        rider_id: Uuid,
        participants: Vec<ParticipantInfo>,
    },

    /// Join rejected response
    JoinRejected {
        session_id: Uuid,
        rider_id: Uuid,
        reason: JoinRejectReason,
    },

    /// Leave notification
    LeaveSession { session_id: Uuid, rider_id: Uuid },

    /// Session ended by host
    SessionEnded { session_id: Uuid },

    /// Real-time metric update
    MetricUpdate {
        session_id: Uuid,
        rider_id: Uuid,
        metrics: RiderMetrics,
        sequence: u32,
    },

    /// Position update (for 3D world)
    PositionUpdate {
        session_id: Uuid,
        rider_id: Uuid,
        position: RiderPosition,
        sequence: u32,
    },

    /// Heartbeat for connection maintenance
    Heartbeat {
        session_id: Uuid,
        rider_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    /// Heartbeat acknowledgment
    HeartbeatAck {
        session_id: Uuid,
        rider_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    /// Chat message
    ChatMessage {
        session_id: Uuid,
        sender_id: Uuid,
        sender_name: String,
        message: String,
        timestamp: DateTime<Utc>,
    },

    /// Participant joined notification
    ParticipantJoined {
        session_id: Uuid,
        participant: ParticipantInfo,
    },

    /// Participant left notification
    ParticipantLeft { session_id: Uuid, rider_id: Uuid },

    /// Race countdown started
    RaceCountdown {
        race_id: Uuid,
        seconds_remaining: u8,
        start_time: DateTime<Utc>,
    },

    /// Race started
    RaceStart {
        race_id: Uuid,
        start_time: DateTime<Utc>,
    },

    /// Activity summary share
    ActivityShare {
        rider_id: Uuid,
        rider_name: String,
        summary: ActivitySummaryData,
    },
}

/// Reason for join rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinRejectReason {
    SessionFull,
    SessionEnded,
    NotAcceptingJoins,
}

/// Participant information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub rider_id: Uuid,
    pub rider_name: String,
    pub avatar_id: Option<String>,
    pub joined_at: DateTime<Utc>,
}

/// Real-time rider metrics for group rides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiderMetrics {
    /// Current power output in watts
    pub power_watts: u16,
    /// Current heart rate in BPM
    pub heart_rate_bpm: Option<u8>,
    /// Current cadence in RPM
    pub cadence_rpm: Option<u8>,
    /// Current speed in km/h
    pub speed_kmh: f32,
    /// Total distance in meters
    pub distance_m: f64,
    /// Elapsed time in milliseconds
    pub elapsed_time_ms: u64,
    /// Current trainer resistance/grade
    pub trainer_grade: Option<f32>,
}

/// Rider position in 3D world.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiderPosition {
    /// Position along route in meters
    pub distance_m: f64,
    /// X coordinate
    pub x: f32,
    /// Y coordinate (elevation)
    pub y: f32,
    /// Z coordinate
    pub z: f32,
    /// Heading in degrees (0-360)
    pub heading: f32,
}

/// Activity summary for sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummaryData {
    pub id: Uuid,
    pub distance_km: f64,
    pub duration_minutes: u32,
    pub avg_power_watts: Option<u16>,
    pub elevation_gain_m: f64,
    pub world_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

impl ProtocolMessage {
    /// Serialize message to bytes using bincode.
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize message from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Get the session ID if applicable.
    pub fn session_id(&self) -> Option<Uuid> {
        match self {
            ProtocolMessage::SessionAnnounce { session_id, .. } => Some(*session_id),
            ProtocolMessage::JoinRequest { session_id, .. } => Some(*session_id),
            ProtocolMessage::JoinAccepted { session_id, .. } => Some(*session_id),
            ProtocolMessage::JoinRejected { session_id, .. } => Some(*session_id),
            ProtocolMessage::LeaveSession { session_id, .. } => Some(*session_id),
            ProtocolMessage::SessionEnded { session_id } => Some(*session_id),
            ProtocolMessage::MetricUpdate { session_id, .. } => Some(*session_id),
            ProtocolMessage::PositionUpdate { session_id, .. } => Some(*session_id),
            ProtocolMessage::Heartbeat { session_id, .. } => Some(*session_id),
            ProtocolMessage::HeartbeatAck { session_id, .. } => Some(*session_id),
            ProtocolMessage::ChatMessage { session_id, .. } => Some(*session_id),
            ProtocolMessage::ParticipantJoined { session_id, .. } => Some(*session_id),
            ProtocolMessage::ParticipantLeft { session_id, .. } => Some(*session_id),
            ProtocolMessage::RaceCountdown { .. } => None,
            ProtocolMessage::RaceStart { .. } => None,
            ProtocolMessage::ActivityShare { .. } => None,
        }
    }

    /// Get the rider ID if applicable.
    pub fn rider_id(&self) -> Option<Uuid> {
        match self {
            ProtocolMessage::SessionAnnounce { host_rider_id, .. } => Some(*host_rider_id),
            ProtocolMessage::JoinRequest { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::JoinAccepted { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::JoinRejected { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::LeaveSession { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::SessionEnded { .. } => None,
            ProtocolMessage::MetricUpdate { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::PositionUpdate { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::Heartbeat { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::HeartbeatAck { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::ChatMessage { sender_id, .. } => Some(*sender_id),
            ProtocolMessage::ParticipantJoined { participant, .. } => Some(participant.rider_id),
            ProtocolMessage::ParticipantLeft { rider_id, .. } => Some(*rider_id),
            ProtocolMessage::RaceCountdown { .. } => None,
            ProtocolMessage::RaceStart { .. } => None,
            ProtocolMessage::ActivityShare { rider_id, .. } => Some(*rider_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_update_serialization() {
        let msg = ProtocolMessage::MetricUpdate {
            session_id: Uuid::new_v4(),
            rider_id: Uuid::new_v4(),
            metrics: RiderMetrics {
                power_watts: 250,
                heart_rate_bpm: Some(145),
                cadence_rpm: Some(90),
                speed_kmh: 35.5,
                distance_m: 10500.0,
                elapsed_time_ms: 1800000,
                trainer_grade: Some(2.5),
            },
            sequence: 1,
        };

        let bytes = msg.to_bytes().unwrap();
        assert!(bytes.len() < MAX_MESSAGE_SIZE);

        let decoded = ProtocolMessage::from_bytes(&bytes).unwrap();
        if let ProtocolMessage::MetricUpdate { metrics, .. } = decoded {
            assert_eq!(metrics.power_watts, 250);
            assert_eq!(metrics.heart_rate_bpm, Some(145));
        } else {
            panic!("Wrong message type");
        }
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ProtocolMessage::ChatMessage {
            session_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            sender_name: "TestRider".to_string(),
            message: "Hello, group!".to_string(),
            timestamp: Utc::now(),
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = ProtocolMessage::from_bytes(&bytes).unwrap();

        if let ProtocolMessage::ChatMessage { message, .. } = decoded {
            assert_eq!(message, "Hello, group!");
        } else {
            panic!("Wrong message type");
        }
    }
}
