//! Group ride chat functionality.
//!
//! Provides real-time chat during group rides.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::protocol::ProtocolMessage;

/// Maximum chat message length.
pub const MAX_MESSAGE_LENGTH: usize = 500;

/// Maximum messages to keep in history.
pub const MAX_HISTORY_SIZE: usize = 100;

/// Chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub is_local: bool,
}

/// Chat service for group rides.
pub struct ChatService {
    session_id: Option<Uuid>,
    local_rider_id: Uuid,
    local_rider_name: String,
    messages: Arc<RwLock<VecDeque<ChatMessage>>>,
    event_tx: broadcast::Sender<ChatMessage>,
}

impl ChatService {
    /// Create a new chat service.
    pub fn new(rider_id: Uuid, rider_name: String) -> Self {
        let (tx, _) = broadcast::channel(64);

        Self {
            session_id: None,
            local_rider_id: rider_id,
            local_rider_name: rider_name,
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            event_tx: tx,
        }
    }

    /// Set the current session.
    pub fn set_session(&mut self, session_id: Uuid) {
        self.session_id = Some(session_id);
        self.messages.write().unwrap().clear();
    }

    /// Clear the current session.
    pub fn clear_session(&mut self) {
        self.session_id = None;
        self.messages.write().unwrap().clear();
    }

    /// Send a chat message.
    pub fn send_message(&self, message: &str) -> Result<ProtocolMessage, ChatError> {
        let session_id = self.session_id.ok_or(ChatError::NotInSession)?;

        if message.is_empty() {
            return Err(ChatError::EmptyMessage);
        }

        let message_text = if message.len() > MAX_MESSAGE_LENGTH {
            &message[..MAX_MESSAGE_LENGTH]
        } else {
            message
        };

        let chat_msg = ChatMessage {
            id: Uuid::new_v4(),
            sender_id: self.local_rider_id,
            sender_name: self.local_rider_name.clone(),
            message: message_text.to_string(),
            timestamp: Utc::now(),
            is_local: true,
        };

        // Add to local history
        self.add_message(chat_msg.clone());

        // Create protocol message for sending
        Ok(ProtocolMessage::ChatMessage {
            session_id,
            sender_id: self.local_rider_id,
            sender_name: self.local_rider_name.clone(),
            message: message_text.to_string(),
            timestamp: chat_msg.timestamp,
        })
    }

    /// Handle a received chat message.
    pub fn receive_message(&self, msg: &ProtocolMessage) {
        if let ProtocolMessage::ChatMessage {
            session_id,
            sender_id,
            sender_name,
            message,
            timestamp,
        } = msg
        {
            // Check session and ignore own messages
            if Some(*session_id) != self.session_id || *sender_id == self.local_rider_id {
                return;
            }

            let chat_msg = ChatMessage {
                id: Uuid::new_v4(),
                sender_id: *sender_id,
                sender_name: sender_name.clone(),
                message: message.clone(),
                timestamp: *timestamp,
                is_local: false,
            };

            self.add_message(chat_msg);
        }
    }

    /// Add a message to history.
    fn add_message(&self, message: ChatMessage) {
        let mut messages = self.messages.write().unwrap();

        // Maintain size limit
        while messages.len() >= MAX_HISTORY_SIZE {
            messages.pop_front();
        }

        messages.push_back(message.clone());

        // Notify subscribers
        let _ = self.event_tx.send(message);
    }

    /// Get chat history.
    pub fn get_history(&self) -> Vec<ChatMessage> {
        self.messages.read().unwrap().iter().cloned().collect()
    }

    /// Get recent messages.
    pub fn get_recent(&self, count: usize) -> Vec<ChatMessage> {
        let messages = self.messages.read().unwrap();
        let start = messages.len().saturating_sub(count);
        messages.iter().skip(start).cloned().collect()
    }

    /// Subscribe to new messages.
    pub fn subscribe(&self) -> broadcast::Receiver<ChatMessage> {
        self.event_tx.subscribe()
    }

    /// Check if in a session.
    pub fn is_active(&self) -> bool {
        self.session_id.is_some()
    }
}

/// Chat errors.
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("Not in a session")]
    NotInSession,

    #[error("Empty message")]
    EmptyMessage,

    #[error("Message too long")]
    MessageTooLong,
}
