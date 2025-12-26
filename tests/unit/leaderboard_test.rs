//! Unit tests for leaderboard queries
//!
//! T066: Unit test for leaderboard queries in tests/unit/leaderboard_test.rs

use rustride::world::segments::SegmentTime;
use rustride::world::segments::leaderboard::{
    LeaderboardEntry, LeaderboardFilter, LeaderboardManager, PersonalRecords,
    SegmentLeaderboard, format_time, format_delta,
};
use chrono::Utc;
use uuid::Uuid;

/// Test time formatting
#[test]
fn test_format_time_basic() {
    assert_eq!(format_time(0.0), "00:00.0");
    assert_eq!(format_time(65.5), "01:05.5");
    assert_eq!(format_time(3600.0), "60:00.0");
}

#[test]
fn test_format_time_edge_cases() {
    assert_eq!(format_time(0.1), "00:00.1");
    assert_eq!(format_time(59.9), "00:59.9");
    assert_eq!(format_time(60.0), "01:00.0");
}

/// Test delta formatting
#[test]
fn test_format_delta_positive() {
    let result = format_delta(5.0);
    assert!(result.starts_with('+'));
    assert!(result.contains("00:05"));
}

#[test]
fn test_format_delta_negative() {
    let result = format_delta(-10.5);
    assert!(result.starts_with('-'));
    assert!(result.contains("00:10"));
}

#[test]
fn test_format_delta_zero() {
    let result = format_delta(0.0);
    assert!(result.starts_with('+')); // Zero is positive
    assert!(result.contains("00:00"));
}

/// Test leaderboard filter variants
#[test]
fn test_leaderboard_filter_default() {
    let filter = LeaderboardFilter::default();
    assert_eq!(filter, LeaderboardFilter::AllTime);
}

/// Test segment leaderboard creation
#[test]
fn test_segment_leaderboard_creation() {
    let segment_id = Uuid::new_v4();
    let leaderboard = SegmentLeaderboard::new(segment_id, "Test Segment".to_string());

    assert_eq!(leaderboard.segment_id, segment_id);
    assert_eq!(leaderboard.segment_name, "Test Segment");
    assert!(leaderboard.entries.is_empty());
    assert_eq!(leaderboard.user_rank, None);
    assert_eq!(leaderboard.total_riders, 0);
    assert_eq!(leaderboard.filter, LeaderboardFilter::AllTime);
}

/// Test leaderboard top entries
#[test]
fn test_leaderboard_top() {
    let segment_id = Uuid::new_v4();
    let mut leaderboard = SegmentLeaderboard::new(segment_id, "Test".to_string());

    // Add some entries
    for i in 0..5 {
        leaderboard.entries.push(LeaderboardEntry {
            rank: i + 1,
            user_id: Uuid::new_v4(),
            user_name: format!("User {}", i),
            time_seconds: 60.0 + i as f64,
            avg_power_watts: Some(200),
            ftp_at_effort: 250,
            recorded_at: Utc::now(),
            is_current_user: false,
        });
    }

    assert_eq!(leaderboard.top(3).len(), 3);
    assert_eq!(leaderboard.top(10).len(), 5); // Only 5 entries
    assert_eq!(leaderboard.top(0).len(), 0);
}

/// Test leaderboard find user
#[test]
fn test_leaderboard_find_user() {
    let segment_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let mut leaderboard = SegmentLeaderboard::new(segment_id, "Test".to_string());

    // Add user entry
    leaderboard.entries.push(LeaderboardEntry {
        rank: 1,
        user_id,
        user_name: "Current User".to_string(),
        time_seconds: 60.0,
        avg_power_watts: Some(200),
        ftp_at_effort: 250,
        recorded_at: Utc::now(),
        is_current_user: true,
    });

    // Add another user
    leaderboard.entries.push(LeaderboardEntry {
        rank: 2,
        user_id: Uuid::new_v4(),
        user_name: "Other User".to_string(),
        time_seconds: 65.0,
        avg_power_watts: Some(190),
        ftp_at_effort: 240,
        recorded_at: Utc::now(),
        is_current_user: false,
    });

    let found = leaderboard.find_user(user_id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().user_name, "Current User");

    let not_found = leaderboard.find_user(Uuid::new_v4());
    assert!(not_found.is_none());
}

/// Test personal records default
#[test]
fn test_personal_records_default() {
    let records = PersonalRecords::default();

    assert!(records.best_time.is_none());
    assert!(records.best_this_year.is_none());
    assert!(records.best_this_month.is_none());
    assert!(records.best_this_week.is_none());
    assert_eq!(records.attempt_count, 0);
    assert!((records.average_time_seconds - 0.0).abs() < 0.01);
}

/// Test leaderboard manager creation
#[test]
fn test_leaderboard_manager_creation() {
    let user_id = Uuid::new_v4();
    let manager = LeaderboardManager::new(user_id);

    // Should have no leaderboards initially
    let segment_id = Uuid::new_v4();
    assert!(manager.get(segment_id).is_none());
}

/// Test leaderboard manager add time
#[test]
fn test_leaderboard_manager_add_time() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    let time = SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 120.0, 250);

    manager.add_time(
        segment_id,
        "Test Segment".to_string(),
        time,
        "Test User".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();
    assert_eq!(leaderboard.entries.len(), 1);
    assert_eq!(leaderboard.entries[0].rank, 1);
    assert!((leaderboard.entries[0].time_seconds - 120.0).abs() < 0.01);
}

/// Test leaderboard manager ranking
#[test]
fn test_leaderboard_manager_ranking() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    // Add times out of order
    let user2 = Uuid::new_v4();
    let user3 = Uuid::new_v4();

    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user2, Uuid::new_v4(), 65.0, 250),
        "User 2".to_string(),
    );

    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 60.0, 250),
        "User 1".to_string(),
    );

    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user3, Uuid::new_v4(), 70.0, 250),
        "User 3".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();

    // Should be sorted by time
    assert_eq!(leaderboard.entries[0].rank, 1);
    assert!((leaderboard.entries[0].time_seconds - 60.0).abs() < 0.01);

    assert_eq!(leaderboard.entries[1].rank, 2);
    assert!((leaderboard.entries[1].time_seconds - 65.0).abs() < 0.01);

    assert_eq!(leaderboard.entries[2].rank, 3);
    assert!((leaderboard.entries[2].time_seconds - 70.0).abs() < 0.01);
}

/// Test user rank tracking
#[test]
fn test_leaderboard_user_rank() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    // Add faster user
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, Uuid::new_v4(), Uuid::new_v4(), 50.0, 250),
        "Fast User".to_string(),
    );

    // Add current user
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 60.0, 250),
        "Current User".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();
    assert_eq!(leaderboard.user_rank, Some(2)); // Should be 2nd place
}

/// Test better time replaces worse time
#[test]
fn test_leaderboard_better_time_replaces() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    // Add initial time
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 70.0, 250),
        "User".to_string(),
    );

    // Add better time
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 60.0, 250),
        "User".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();
    assert_eq!(leaderboard.entries.len(), 1); // Still just one entry
    assert!((leaderboard.entries[0].time_seconds - 60.0).abs() < 0.01); // Better time
}

/// Test worse time doesn't replace better
#[test]
fn test_leaderboard_worse_time_ignored() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    // Add good time
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 60.0, 250),
        "User".to_string(),
    );

    // Add worse time
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 70.0, 250),
        "User".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();
    assert_eq!(leaderboard.entries.len(), 1);
    assert!((leaderboard.entries[0].time_seconds - 60.0).abs() < 0.01); // Original good time
}

/// Test personal records tracking
#[test]
fn test_personal_records_tracking() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    // Add time
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 60.0, 250),
        "User".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();
    assert_eq!(leaderboard.personal_records.attempt_count, 1);
    assert!(leaderboard.personal_records.best_time.is_some());
}

/// Test multiple segments
#[test]
fn test_leaderboard_multiple_segments() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);

    let segment1 = Uuid::new_v4();
    let segment2 = Uuid::new_v4();

    manager.add_time(
        segment1,
        "Segment 1".to_string(),
        SegmentTime::new(segment1, user_id, Uuid::new_v4(), 60.0, 250),
        "User".to_string(),
    );

    manager.add_time(
        segment2,
        "Segment 2".to_string(),
        SegmentTime::new(segment2, user_id, Uuid::new_v4(), 90.0, 250),
        "User".to_string(),
    );

    assert!(manager.get(segment1).is_some());
    assert!(manager.get(segment2).is_some());
    assert!((manager.get(segment1).unwrap().entries[0].time_seconds - 60.0).abs() < 0.01);
    assert!((manager.get(segment2).unwrap().entries[0].time_seconds - 90.0).abs() < 0.01);
}

/// Test load segment
#[test]
fn test_load_segment() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    manager.load_segment(segment_id, "New Segment".to_string());

    assert!(manager.get(segment_id).is_some());
    assert_eq!(manager.get(segment_id).unwrap().segment_name, "New Segment");
}

/// Test clear leaderboards
#[test]
fn test_clear_leaderboards() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 60.0, 250),
        "User".to_string(),
    );

    assert!(manager.get(segment_id).is_some());

    manager.clear();

    assert!(manager.get(segment_id).is_none());
}

/// Test total riders count
#[test]
fn test_total_riders_count() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    for i in 0..5 {
        let rider_id = Uuid::new_v4();
        manager.add_time(
            segment_id,
            "Test".to_string(),
            SegmentTime::new(segment_id, rider_id, Uuid::new_v4(), 60.0 + i as f64, 250),
            format!("User {}", i),
        );
    }

    let leaderboard = manager.get(segment_id).unwrap();
    assert_eq!(leaderboard.total_riders, 5);
}

/// Test is_current_user flag
#[test]
fn test_is_current_user_flag() {
    let user_id = Uuid::new_v4();
    let mut manager = LeaderboardManager::new(user_id);
    let segment_id = Uuid::new_v4();

    // Add other user
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, Uuid::new_v4(), Uuid::new_v4(), 60.0, 250),
        "Other".to_string(),
    );

    // Add current user
    manager.add_time(
        segment_id,
        "Test".to_string(),
        SegmentTime::new(segment_id, user_id, Uuid::new_v4(), 70.0, 250),
        "Current".to_string(),
    );

    let leaderboard = manager.get(segment_id).unwrap();

    let other_entry = leaderboard.entries.iter().find(|e| e.user_name == "Other").unwrap();
    assert!(!other_entry.is_current_user);

    let current_entry = leaderboard.entries.iter().find(|e| e.user_name == "Current").unwrap();
    assert!(current_entry.is_current_user);
}
