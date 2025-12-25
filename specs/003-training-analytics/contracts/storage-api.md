# Analytics Storage API Contract

**Module**: `src/storage/analytics_store.rs`
**Feature**: 003-training-analytics
**Date**: 2025-12-25

## Overview

This contract defines the storage layer API for persisting analytics data. Extends existing `database.rs` with analytics-specific operations.

---

## Module Structure

```rust
// src/storage/analytics_store.rs
use crate::metrics::analytics::*;
use rusqlite::Connection;

pub struct AnalyticsStore<'a> {
    conn: &'a Connection,
}
```

---

## Power Duration Curve Storage

```rust
impl<'a> AnalyticsStore<'a> {
    /// Load user's PDC from database
    pub fn load_pdc(&self, user_id: &str) -> AnalyticsResult<PowerDurationCurve>;

    /// Load PDC filtered by date range
    pub fn load_pdc_range(
        &self,
        user_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AnalyticsResult<PowerDurationCurve>;

    /// Save or update PDC points
    /// Returns number of points updated
    pub fn save_pdc_points(
        &self,
        user_id: &str,
        points: &[PdcPoint],
        ride_id: Option<&str>,
    ) -> AnalyticsResult<usize>;

    /// Delete PDC points (for recalculation)
    pub fn clear_pdc(&self, user_id: &str) -> AnalyticsResult<()>;
}
```

---

## Critical Power Model Storage

```rust
impl<'a> AnalyticsStore<'a> {
    /// Load current CP model for user
    pub fn load_current_cp_model(&self, user_id: &str) -> AnalyticsResult<Option<CpModel>>;

    /// Load CP model history
    pub fn load_cp_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> AnalyticsResult<Vec<(DateTime<Utc>, CpModel)>>;

    /// Save new CP model (marks previous as not current)
    pub fn save_cp_model(
        &self,
        user_id: &str,
        model: &CpModel,
        fitting_points: &[(u32, u16)],
    ) -> AnalyticsResult<String>; // Returns model ID
}
```

---

## FTP Estimate Storage

```rust
impl<'a> AnalyticsStore<'a> {
    /// Load current accepted FTP for user
    pub fn load_accepted_ftp(&self, user_id: &str) -> AnalyticsResult<Option<u16>>;

    /// Load pending (unaccepted) FTP estimate
    pub fn load_pending_ftp(&self, user_id: &str) -> AnalyticsResult<Option<FtpEstimate>>;

    /// Load FTP history
    pub fn load_ftp_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> AnalyticsResult<Vec<FtpEstimate>>;

    /// Save new FTP estimate (pending acceptance)
    pub fn save_ftp_estimate(
        &self,
        user_id: &str,
        estimate: &FtpEstimate,
    ) -> AnalyticsResult<String>; // Returns estimate ID

    /// Accept FTP estimate (updates user profile FTP)
    pub fn accept_ftp(&self, user_id: &str, estimate_id: &str) -> AnalyticsResult<()>;

    /// Dismiss FTP estimate without accepting
    pub fn dismiss_ftp(&self, estimate_id: &str) -> AnalyticsResult<()>;
}
```

---

## Training Load Storage

```rust
impl<'a> AnalyticsStore<'a> {
    /// Load training load for specific date
    pub fn load_daily_load(
        &self,
        user_id: &str,
        date: NaiveDate,
    ) -> AnalyticsResult<Option<DailyLoad>>;

    /// Load training load history
    pub fn load_load_history(
        &self,
        user_id: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> AnalyticsResult<Vec<(NaiveDate, DailyLoad)>>;

    /// Save daily training load
    pub fn save_daily_load(
        &self,
        user_id: &str,
        date: NaiveDate,
        load: &DailyLoad,
        ride_count: u32,
        total_duration: u32,
    ) -> AnalyticsResult<()>;

    /// Recalculate training load from scratch
    /// (used after TSS corrections or ride deletions)
    pub fn recalculate_load_history(
        &self,
        user_id: &str,
        from_date: NaiveDate,
    ) -> AnalyticsResult<()>;

    /// Get current ACWR for user
    pub fn current_acwr(&self, user_id: &str) -> AnalyticsResult<Option<Acwr>>;
}
```

---

## VO2max Storage

```rust
impl<'a> AnalyticsStore<'a> {
    /// Load most recent VO2max estimate
    pub fn load_current_vo2max(&self, user_id: &str) -> AnalyticsResult<Option<Vo2maxResult>>;

    /// Load VO2max history
    pub fn load_vo2max_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> AnalyticsResult<Vec<(DateTime<Utc>, Vo2maxResult)>>;

    /// Save new VO2max estimate
    pub fn save_vo2max(
        &self,
        user_id: &str,
        result: &Vo2maxResult,
    ) -> AnalyticsResult<String>;
}
```

---

## Rider Profile Storage

```rust
impl<'a> AnalyticsStore<'a> {
    /// Load rider analytics profile
    pub fn load_rider_profile(
        &self,
        user_id: &str,
    ) -> AnalyticsResult<Option<(RiderType, PowerProfile)>>;

    /// Save rider classification
    pub fn save_rider_profile(
        &self,
        user_id: &str,
        rider_type: RiderType,
        profile: &PowerProfile,
    ) -> AnalyticsResult<()>;
}
```

---

## Aggregation Queries

```rust
impl<'a> AnalyticsStore<'a> {
    /// Get daily TSS totals for date range
    /// (aggregates from rides table)
    pub fn aggregate_daily_tss(
        &self,
        user_id: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> AnalyticsResult<Vec<(NaiveDate, f32)>>;

    /// Get ride samples for MMP calculation
    pub fn load_ride_power_samples(
        &self,
        ride_id: &str,
    ) -> AnalyticsResult<Vec<u16>>;

    /// Get all rides in date range for PDC calculation
    pub fn load_rides_for_pdc(
        &self,
        user_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> AnalyticsResult<Vec<String>>; // Returns ride IDs
}
```

---

## Transaction Support

```rust
impl<'a> AnalyticsStore<'a> {
    /// Execute analytics update in transaction
    /// (for atomic PDC + CP + FTP update after ride)
    pub fn with_transaction<F, T>(&self, f: F) -> AnalyticsResult<T>
    where
        F: FnOnce(&AnalyticsStore) -> AnalyticsResult<T>;
}
```

---

## Example Usage

```rust
// After a ride is saved, update all analytics
fn update_analytics_after_ride(
    conn: &Connection,
    user_id: &str,
    ride_id: &str,
) -> AnalyticsResult<()> {
    let store = AnalyticsStore::new(conn);

    store.with_transaction(|store| {
        // 1. Load ride samples and calculate MMP
        let samples = store.load_ride_power_samples(ride_id)?;
        let mmp = MmpCalculator::standard().calculate(&samples);

        // 2. Update PDC
        let mut pdc = store.load_pdc(user_id)?;
        let updated = pdc.update(&mmp);
        if !updated.is_empty() {
            store.save_pdc_points(user_id, &updated, Some(ride_id))?;
        }

        // 3. Recalculate CP if significant PDC changes
        if pdc.has_sufficient_data_for_cp() {
            if let Ok(model) = CpFitter::new().fit(&pdc) {
                store.save_cp_model(user_id, &model, &[])?;
            }
        }

        // 4. Check FTP
        let detector = FtpDetector::new();
        if let Some(estimate) = detector.detect(&pdc) {
            let current_ftp = store.load_accepted_ftp(user_id)?;
            if let Some(current) = current_ftp {
                if detector.is_significant_change(current, &estimate) {
                    store.save_ftp_estimate(user_id, &estimate)?;
                }
            }
        }

        // 5. Update training load
        let today = Utc::now().date_naive();
        let ride_tss = // get from ride...
        store.recalculate_load_history(user_id, today)?;

        Ok(())
    })
}
```

---

## Migration Functions

```rust
impl<'a> AnalyticsStore<'a> {
    /// Run schema migrations for analytics tables
    pub fn migrate(&self) -> AnalyticsResult<()>;

    /// Check if analytics tables exist
    pub fn is_initialized(&self) -> bool;

    /// Rebuild all analytics from ride data
    /// (for migration from older versions)
    pub fn rebuild_all_analytics(&self, user_id: &str) -> AnalyticsResult<()>;
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Data integrity error: {0}")]
    IntegrityError(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}
```
