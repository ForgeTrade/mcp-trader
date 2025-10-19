//! HTTP session management for MCP transport
//!
//! Manages stateful HTTP sessions with:
//! - 30-minute idle timeout
//! - 50 concurrent session limit
//! - UUID-based session identification
//! - Automatic cleanup of expired sessions

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Session metadata for HTTP transport
#[derive(Debug, Clone)]
pub struct StreamableHttpSession {
    /// Unique session identifier (UUID v4)
    pub session_id: Uuid,

    /// Client metadata (User-Agent, IP, etc.)
    pub client_metadata: HashMap<String, String>,

    /// Session creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp (updated on each request)
    pub last_activity: DateTime<Utc>,

    /// Session expiration timestamp (created_at + 30 minutes)
    pub expires_at: DateTime<Utc>,
}

impl StreamableHttpSession {
    /// Create a new session
    pub fn new(client_metadata: HashMap<String, String>) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::minutes(30);

        Self {
            session_id: Uuid::new_v4(),
            client_metadata,
            created_at: now,
            last_activity: now,
            expires_at,
        }
    }

    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Update last activity timestamp and extend expiration
    pub fn touch(&mut self) {
        let now = Utc::now();
        self.last_activity = now;
        self.expires_at = now + Duration::minutes(30);
    }
}

/// Thread-safe session store
#[derive(Clone)]
pub struct SessionStore {
    /// Active sessions keyed by session ID
    sessions: Arc<RwLock<HashMap<Uuid, StreamableHttpSession>>>,

    /// Maximum concurrent sessions (default: 50)
    max_sessions: usize,
}

impl SessionStore {
    /// Create a new session store
    pub fn new(max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions,
        }
    }

    /// Create a new session and store it
    ///
    /// # Arguments
    /// * `client_metadata` - Client information (User-Agent, IP, etc.)
    ///
    /// # Returns
    /// Session ID if successful, or error if session limit reached
    ///
    /// # Errors
    /// - `SessionLimitExceeded` if max_sessions reached (default: 50)
    pub fn create_session(
        &self,
        client_metadata: HashMap<String, String>,
    ) -> Result<Uuid, SessionError> {
        let mut sessions = self.sessions.write().unwrap();

        // Check session limit
        if sessions.len() >= self.max_sessions {
            return Err(SessionError::SessionLimitExceeded(self.max_sessions));
        }

        let session = StreamableHttpSession::new(client_metadata);
        let session_id = session.session_id;

        sessions.insert(session_id, session);

        Ok(session_id)
    }

    /// Validate session and update activity timestamp
    ///
    /// # Arguments
    /// * `session_id` - Session UUID from Mcp-Session-Id header
    ///
    /// # Returns
    /// Ok(()) if session is valid and not expired
    ///
    /// # Errors
    /// - `SessionNotFound` if session ID doesn't exist
    /// - `SessionExpired` if session exceeded 30-minute timeout
    pub fn validate_session(&self, session_id: Uuid) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().unwrap();

        match sessions.get_mut(&session_id) {
            Some(session) => {
                if session.is_expired() {
                    sessions.remove(&session_id);
                    Err(SessionError::SessionExpired(session_id))
                } else {
                    session.touch();
                    Ok(())
                }
            }
            None => Err(SessionError::SessionNotFound(session_id)),
        }
    }

    /// Get session metadata (read-only)
    pub fn get_session(&self, session_id: Uuid) -> Option<StreamableHttpSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(&session_id).cloned()
    }

    /// Remove expired sessions (should be called periodically)
    ///
    /// # Returns
    /// Number of sessions removed
    pub fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().unwrap();
        let now = Utc::now();

        let expired_ids: Vec<Uuid> = sessions
            .iter()
            .filter(|(_, session)| session.expires_at < now)
            .map(|(id, _)| *id)
            .collect();

        for id in &expired_ids {
            sessions.remove(id);
        }

        expired_ids.len()
    }

    /// Get current session count
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Session-related errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Session expired: {0}")]
    SessionExpired(Uuid),

    #[error("Session limit exceeded: maximum {0} concurrent sessions")]
    SessionLimitExceeded(usize),

    #[error("Invalid session ID format")]
    InvalidSessionId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let store = SessionStore::new(50);
        let metadata = HashMap::new();

        let session_id = store.create_session(metadata).unwrap();
        assert_eq!(store.session_count(), 1);

        let session = store.get_session(session_id).unwrap();
        assert_eq!(session.session_id, session_id);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_limit() {
        let store = SessionStore::new(2);

        let _ = store.create_session(HashMap::new()).unwrap();
        let _ = store.create_session(HashMap::new()).unwrap();

        // Third session should fail
        let result = store.create_session(HashMap::new());
        assert!(matches!(result, Err(SessionError::SessionLimitExceeded(2))));
    }

    #[test]
    fn test_validate_session() {
        let store = SessionStore::new(50);
        let session_id = store.create_session(HashMap::new()).unwrap();

        // Should validate successfully
        assert!(store.validate_session(session_id).is_ok());

        // Invalid session should fail
        let fake_id = Uuid::new_v4();
        assert!(matches!(
            store.validate_session(fake_id),
            Err(SessionError::SessionNotFound(_))
        ));
    }

    #[test]
    fn test_session_touch() {
        let store = SessionStore::new(50);
        let session_id = store.create_session(HashMap::new()).unwrap();

        let original_expiry = store.get_session(session_id).unwrap().expires_at;

        // Validate (which calls touch)
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.validate_session(session_id).unwrap();

        let new_expiry = store.get_session(session_id).unwrap().expires_at;
        assert!(new_expiry > original_expiry);
    }
}
