//! Session management for group rides.
//!
//! Handles hosting and joining group ride sessions.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::discovery::PeerInfo;
use super::protocol::{JoinRejectReason, ParticipantInfo};

/// Maximum participants per session.
pub const MAX_PARTICIPANTS: usize = 10;

/// Heartbeat interval in milliseconds.
pub const HEARTBEAT_INTERVAL_MS: u64 = 1000;

/// Disconnect timeout in milliseconds.
pub const DISCONNECT_TIMEOUT_MS: u64 = 5000;

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Not in any session.
    Idle,
    /// Hosting a session.
    Hosting,
    /// Joined another session.
    Joined,
}

/// Group ride session.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub host_rider_id: Uuid,
    pub host_name: String,
    pub name: Option<String>,
    pub world_id: String,
    pub created_at: DateTime<Utc>,
    pub max_participants: u8,
}

/// Session participant.
#[derive(Debug, Clone)]
pub struct Participant {
    pub rider_id: Uuid,
    pub rider_name: String,
    pub avatar_id: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub is_host: bool,
}

/// Session event.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Session created.
    SessionCreated(Session),
    /// Joined a session.
    SessionJoined(Session),
    /// Session ended.
    SessionEnded { session_id: Uuid },
    /// Participant joined.
    ParticipantJoined(Participant),
    /// Participant left.
    ParticipantLeft { rider_id: Uuid },
    /// Join request received (host only).
    JoinRequest { rider_id: Uuid, rider_name: String },
    /// Join was rejected.
    JoinRejected { reason: JoinRejectReason },
}

/// Session manager.
pub struct SessionManager {
    local_rider_id: Uuid,
    local_rider_name: String,
    state: Arc<RwLock<SessionState>>,
    current_session: Arc<RwLock<Option<Session>>>,
    participants: Arc<RwLock<HashMap<Uuid, Participant>>>,
    event_tx: broadcast::Sender<SessionEvent>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(rider_id: Uuid, rider_name: String) -> Self {
        let (tx, _) = broadcast::channel(64);

        Self {
            local_rider_id: rider_id,
            local_rider_name: rider_name,
            state: Arc::new(RwLock::new(SessionState::Idle)),
            current_session: Arc::new(RwLock::new(None)),
            participants: Arc::new(RwLock::new(HashMap::new())),
            event_tx: tx,
        }
    }

    /// Get current session state.
    pub fn state(&self) -> SessionState {
        *self.state.read().unwrap()
    }

    /// Get current session if any.
    pub fn current_session(&self) -> Option<Session> {
        self.current_session.read().unwrap().clone()
    }

    /// Get all participants in current session.
    pub fn participants(&self) -> Vec<Participant> {
        self.participants
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Host a new session.
    pub fn host_session(
        &self,
        name: Option<String>,
        world_id: String,
    ) -> Result<Session, SessionError> {
        let mut state = self.state.write().unwrap();

        if *state != SessionState::Idle {
            return Err(SessionError::AlreadyInSession);
        }

        let session = Session {
            id: Uuid::new_v4(),
            host_rider_id: self.local_rider_id,
            host_name: self.local_rider_name.clone(),
            name,
            world_id,
            created_at: Utc::now(),
            max_participants: MAX_PARTICIPANTS as u8,
        };

        // Add self as participant
        let host_participant = Participant {
            rider_id: self.local_rider_id,
            rider_name: self.local_rider_name.clone(),
            avatar_id: None,
            joined_at: Utc::now(),
            last_seen: Utc::now(),
            is_host: true,
        };

        self.participants
            .write()
            .unwrap()
            .insert(self.local_rider_id, host_participant);

        *self.current_session.write().unwrap() = Some(session.clone());
        *state = SessionState::Hosting;

        let _ = self
            .event_tx
            .send(SessionEvent::SessionCreated(session.clone()));

        Ok(session)
    }

    /// Join an existing session.
    pub fn join_session(&self, peer: &PeerInfo, session_id: Uuid) -> Result<(), SessionError> {
        let mut state = self.state.write().unwrap();

        if *state != SessionState::Idle {
            return Err(SessionError::AlreadyInSession);
        }

        // Create a pending session (will be confirmed when host accepts)
        let session = Session {
            id: session_id,
            host_rider_id: peer.rider_id,
            host_name: peer.rider_name.clone(),
            name: None,
            world_id: peer.world_id.clone().unwrap_or_default(),
            created_at: Utc::now(),
            max_participants: MAX_PARTICIPANTS as u8,
        };

        *self.current_session.write().unwrap() = Some(session.clone());
        *state = SessionState::Joined;

        // Add self as participant
        let self_participant = Participant {
            rider_id: self.local_rider_id,
            rider_name: self.local_rider_name.clone(),
            avatar_id: None,
            joined_at: Utc::now(),
            last_seen: Utc::now(),
            is_host: false,
        };

        self.participants
            .write()
            .unwrap()
            .insert(self.local_rider_id, self_participant);

        let _ = self.event_tx.send(SessionEvent::SessionJoined(session));

        Ok(())
    }

    /// Leave the current session.
    pub fn leave_session(&self) -> Result<(), SessionError> {
        let mut state = self.state.write().unwrap();

        if *state == SessionState::Idle {
            return Err(SessionError::NotInSession);
        }

        let session_id = self.current_session.read().unwrap().as_ref().map(|s| s.id);

        self.participants.write().unwrap().clear();
        *self.current_session.write().unwrap() = None;
        *state = SessionState::Idle;

        if let Some(id) = session_id {
            let _ = self
                .event_tx
                .send(SessionEvent::SessionEnded { session_id: id });
        }

        Ok(())
    }

    /// Handle an incoming join request (host only).
    pub fn handle_join_request(
        &self,
        rider_id: Uuid,
        rider_name: String,
    ) -> Result<bool, SessionError> {
        let state = self.state.read().unwrap();

        if *state != SessionState::Hosting {
            return Err(SessionError::NotHosting);
        }

        let participants = self.participants.read().unwrap();

        // Check if session is full
        if participants.len() >= MAX_PARTICIPANTS {
            return Ok(false);
        }

        // Check if already joined
        if participants.contains_key(&rider_id) {
            return Ok(true);
        }

        drop(participants);

        // Add participant
        let participant = Participant {
            rider_id,
            rider_name: rider_name.clone(),
            avatar_id: None,
            joined_at: Utc::now(),
            last_seen: Utc::now(),
            is_host: false,
        };

        self.participants
            .write()
            .unwrap()
            .insert(rider_id, participant.clone());

        let _ = self
            .event_tx
            .send(SessionEvent::ParticipantJoined(participant));

        Ok(true)
    }

    /// Handle join accepted (client only).
    pub fn handle_join_accepted(&self, participants: Vec<ParticipantInfo>) {
        let mut parts = self.participants.write().unwrap();

        for info in participants {
            if info.rider_id != self.local_rider_id {
                let participant = Participant {
                    rider_id: info.rider_id,
                    rider_name: info.rider_name,
                    avatar_id: info.avatar_id,
                    joined_at: info.joined_at,
                    last_seen: Utc::now(),
                    is_host: false,
                };
                parts.insert(info.rider_id, participant);
            }
        }
    }

    /// Handle participant joined notification.
    pub fn handle_participant_joined(&self, info: ParticipantInfo) {
        if info.rider_id == self.local_rider_id {
            return;
        }

        let participant = Participant {
            rider_id: info.rider_id,
            rider_name: info.rider_name,
            avatar_id: info.avatar_id,
            joined_at: info.joined_at,
            last_seen: Utc::now(),
            is_host: false,
        };

        self.participants
            .write()
            .unwrap()
            .insert(info.rider_id, participant.clone());

        let _ = self
            .event_tx
            .send(SessionEvent::ParticipantJoined(participant));
    }

    /// Handle participant left notification.
    pub fn handle_participant_left(&self, rider_id: Uuid) {
        if self
            .participants
            .write()
            .unwrap()
            .remove(&rider_id)
            .is_some()
        {
            let _ = self
                .event_tx
                .send(SessionEvent::ParticipantLeft { rider_id });
        }
    }

    /// Update participant's last seen time.
    pub fn update_participant_heartbeat(&self, rider_id: Uuid) {
        if let Some(participant) = self.participants.write().unwrap().get_mut(&rider_id) {
            participant.last_seen = Utc::now();
        }
    }

    /// Check for disconnected participants.
    pub fn check_disconnects(&self) -> Vec<Uuid> {
        let now = Utc::now();
        let timeout = chrono::Duration::milliseconds(DISCONNECT_TIMEOUT_MS as i64);

        let disconnected: Vec<Uuid> = self
            .participants
            .read()
            .unwrap()
            .iter()
            .filter(|(id, p)| **id != self.local_rider_id && (now - p.last_seen) > timeout)
            .map(|(id, _)| *id)
            .collect();

        for rider_id in &disconnected {
            self.handle_participant_left(*rider_id);
        }

        disconnected
    }

    /// Subscribe to session events.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }

    /// Get participant info for protocol messages.
    pub fn get_participant_infos(&self) -> Vec<ParticipantInfo> {
        self.participants
            .read()
            .unwrap()
            .values()
            .map(|p| ParticipantInfo {
                rider_id: p.rider_id,
                rider_name: p.rider_name.clone(),
                avatar_id: p.avatar_id.clone(),
                joined_at: p.joined_at,
            })
            .collect()
    }
}

/// Session errors.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Already in a session")]
    AlreadyInSession,

    #[error("Not in a session")]
    NotInSession,

    #[error("Not hosting a session")]
    NotHosting,

    #[error("Session is full")]
    SessionFull,

    #[error("Session not found")]
    SessionNotFound,
}
