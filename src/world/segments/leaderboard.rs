//! Segment leaderboard management and display.

use super::SegmentTime;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Time range filter for leaderboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeaderboardFilter {
    #[default]
    /// All time best
    AllTime,
    /// This year
    ThisYear,
    /// This month
    ThisMonth,
    /// This week
    ThisWeek,
    /// Today only
    Today,
}

/// Leaderboard entry (combined personal and global)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Rank on leaderboard (1-indexed)
    pub rank: u32,
    /// User ID
    pub user_id: Uuid,
    /// Display name
    pub user_name: String,
    /// Time in seconds
    pub time_seconds: f64,
    /// Average power (if available)
    pub avg_power_watts: Option<u16>,
    /// FTP at time of effort
    pub ftp_at_effort: u16,
    /// When recorded
    pub recorded_at: DateTime<Utc>,
    /// Whether this is the current user
    pub is_current_user: bool,
}

/// Personal records on a segment
#[derive(Debug, Clone, Default)]
pub struct PersonalRecords {
    /// Best time ever
    pub best_time: Option<SegmentTime>,
    /// Best time this year
    pub best_this_year: Option<SegmentTime>,
    /// Best time this month
    pub best_this_month: Option<SegmentTime>,
    /// Best time this week
    pub best_this_week: Option<SegmentTime>,
    /// Total attempts
    pub attempt_count: u32,
    /// Average time
    pub average_time_seconds: f64,
}

/// Leaderboard for a single segment
#[derive(Debug, Clone)]
pub struct SegmentLeaderboard {
    /// Segment ID
    pub segment_id: Uuid,
    /// Segment name
    pub segment_name: String,
    /// Top entries (limited)
    pub entries: Vec<LeaderboardEntry>,
    /// Current user's position (if on board)
    pub user_rank: Option<u32>,
    /// Current user's personal records
    pub personal_records: PersonalRecords,
    /// Total riders on this segment
    pub total_riders: u32,
    /// Active filter
    pub filter: LeaderboardFilter,
}

impl SegmentLeaderboard {
    /// Create empty leaderboard
    pub fn new(segment_id: Uuid, segment_name: String) -> Self {
        Self {
            segment_id,
            segment_name,
            entries: Vec::new(),
            user_rank: None,
            personal_records: PersonalRecords::default(),
            total_riders: 0,
            filter: LeaderboardFilter::AllTime,
        }
    }

    /// Get top N entries
    pub fn top(&self, n: usize) -> &[LeaderboardEntry] {
        let end = n.min(self.entries.len());
        &self.entries[..end]
    }

    /// Find entry by user ID
    pub fn find_user(&self, user_id: Uuid) -> Option<&LeaderboardEntry> {
        self.entries.iter().find(|e| e.user_id == user_id)
    }
}

/// Leaderboard manager handles all segment leaderboards
pub struct LeaderboardManager {
    /// Currently loaded leaderboards (segment_id -> leaderboard)
    leaderboards: std::collections::HashMap<Uuid, SegmentLeaderboard>,
    /// Current user ID
    user_id: Uuid,
    /// Maximum entries to keep per leaderboard
    max_entries: usize,
}

impl LeaderboardManager {
    /// Create leaderboard manager
    pub fn new(user_id: Uuid) -> Self {
        Self {
            leaderboards: std::collections::HashMap::new(),
            user_id,
            max_entries: 100,
        }
    }

    /// Add or update a time on a leaderboard
    pub fn add_time(
        &mut self,
        segment_id: Uuid,
        segment_name: String,
        time: SegmentTime,
        user_name: String,
    ) {
        let leaderboard = self
            .leaderboards
            .entry(segment_id)
            .or_insert_with(|| SegmentLeaderboard::new(segment_id, segment_name));

        // Create entry
        let entry = LeaderboardEntry {
            rank: 0, // Will be recalculated
            user_id: time.user_id,
            user_name,
            time_seconds: time.time_seconds,
            avg_power_watts: time.avg_power_watts,
            ftp_at_effort: time.ftp_at_effort,
            recorded_at: time.recorded_at,
            is_current_user: time.user_id == self.user_id,
        };

        // Check if user already has an entry
        if let Some(existing) = leaderboard
            .entries
            .iter_mut()
            .find(|e| e.user_id == time.user_id)
        {
            // Only update if new time is better
            if time.time_seconds < existing.time_seconds {
                *existing = entry;
            }
        } else {
            leaderboard.entries.push(entry);
            leaderboard.total_riders += 1;
        }

        // Re-sort and re-rank
        self.recalculate_ranks(segment_id);

        // Update personal records if current user
        if time.user_id == self.user_id {
            self.update_personal_records(segment_id, time);
        }
    }

    /// Recalculate ranks after changes
    fn recalculate_ranks(&mut self, segment_id: Uuid) {
        if let Some(leaderboard) = self.leaderboards.get_mut(&segment_id) {
            // Sort by time (ascending)
            leaderboard
                .entries
                .sort_by(|a, b| a.time_seconds.partial_cmp(&b.time_seconds).unwrap());

            // Truncate to max entries
            leaderboard.entries.truncate(self.max_entries);

            // Assign ranks
            for (i, entry) in leaderboard.entries.iter_mut().enumerate() {
                entry.rank = (i + 1) as u32;
            }

            // Update user rank
            leaderboard.user_rank = leaderboard
                .entries
                .iter()
                .find(|e| e.is_current_user)
                .map(|e| e.rank);
        }
    }

    /// Update personal records for a segment
    fn update_personal_records(&mut self, segment_id: Uuid, time: SegmentTime) {
        if let Some(leaderboard) = self.leaderboards.get_mut(&segment_id) {
            let pr = &mut leaderboard.personal_records;
            pr.attempt_count += 1;

            // Update running average
            let n = pr.attempt_count as f64;
            pr.average_time_seconds =
                ((pr.average_time_seconds * (n - 1.0)) + time.time_seconds) / n;

            // Check if new best
            if pr.best_time.is_none()
                || time.time_seconds < pr.best_time.as_ref().unwrap().time_seconds
            {
                pr.best_time = Some(time.clone());
            }

            // Time-filtered bests would require date checking
            // For now, just update if better (simplified)
            if pr.best_this_year.is_none()
                || time.time_seconds < pr.best_this_year.as_ref().unwrap().time_seconds
            {
                pr.best_this_year = Some(time.clone());
            }

            if pr.best_this_month.is_none()
                || time.time_seconds < pr.best_this_month.as_ref().unwrap().time_seconds
            {
                pr.best_this_month = Some(time.clone());
            }

            if pr.best_this_week.is_none()
                || time.time_seconds < pr.best_this_week.as_ref().unwrap().time_seconds
            {
                pr.best_this_week = Some(time);
            }
        }
    }

    /// Get leaderboard for a segment
    pub fn get(&self, segment_id: Uuid) -> Option<&SegmentLeaderboard> {
        self.leaderboards.get(&segment_id)
    }

    /// Get mutable leaderboard
    pub fn get_mut(&mut self, segment_id: Uuid) -> Option<&mut SegmentLeaderboard> {
        self.leaderboards.get_mut(&segment_id)
    }

    /// Load leaderboard from database (stub - would query storage)
    pub fn load_segment(&mut self, segment_id: Uuid, segment_name: String) {
        self.leaderboards
            .entry(segment_id)
            .or_insert_with(|| SegmentLeaderboard::new(segment_id, segment_name));
    }

    /// Clear all leaderboards
    pub fn clear(&mut self) {
        self.leaderboards.clear();
    }
}

/// Format time as MM:SS.t
pub fn format_time(seconds: f64) -> String {
    let minutes = (seconds / 60.0).floor() as u32;
    let secs = seconds % 60.0;
    format!("{:02}:{:04.1}", minutes, secs)
}

/// Format time delta with sign
pub fn format_delta(seconds: f64) -> String {
    let sign = if seconds < 0.0 { "-" } else { "+" };
    let abs_secs = seconds.abs();
    format!("{}{}", sign, format_time(abs_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(65.5), "01:05.5");
        assert_eq!(format_time(0.0), "00:00.0");
        assert_eq!(format_time(3599.9), "59:59.9");
    }

    #[test]
    fn test_format_delta() {
        assert_eq!(format_delta(-5.0), "-00:05.0");
        assert_eq!(format_delta(10.0), "+00:10.0");
    }

    #[test]
    fn test_leaderboard_manager() {
        let user_id = Uuid::new_v4();
        let mut manager = LeaderboardManager::new(user_id);
        let segment_id = Uuid::new_v4();

        // Add time
        let time = super::super::SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 120.0, 250);

        manager.add_time(
            segment_id,
            "Test Segment".to_string(),
            time,
            "Test User".to_string(),
        );

        let lb = manager.get(segment_id).unwrap();
        assert_eq!(lb.entries.len(), 1);
        assert_eq!(lb.entries[0].rank, 1);
    }
}
