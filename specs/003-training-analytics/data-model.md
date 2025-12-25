# Data Model: Training Science & Analytics

**Feature**: 003-training-analytics
**Date**: 2025-12-25

## Overview

This document defines the data entities required for the Training Science & Analytics feature. Entities extend the existing schema in `src/storage/schema.rs`.

---

## New Entities

### 1. PowerDurationCurve

Stores the user's maximum power at each duration across all rides.

```rust
/// Power Duration Curve entry
pub struct PdcEntry {
    /// User ID (foreign key to users table)
    pub user_id: String,
    /// Duration in seconds
    pub duration_seconds: u32,
    /// Maximum power achieved at this duration (watts)
    pub max_power: u16,
    /// When this max power was achieved
    pub achieved_at: DateTime<Utc>,
    /// Which ride achieved this max (for reference)
    pub ride_id: Option<String>,
}
```

**SQL Schema**:
```sql
CREATE TABLE IF NOT EXISTS power_duration_curve (
    user_id TEXT NOT NULL REFERENCES users(id),
    duration_seconds INTEGER NOT NULL,
    max_power INTEGER NOT NULL,
    achieved_at TEXT NOT NULL,
    ride_id TEXT REFERENCES rides(id) ON DELETE SET NULL,
    PRIMARY KEY (user_id, duration_seconds)
);

CREATE INDEX IF NOT EXISTS idx_pdc_user_id ON power_duration_curve(user_id);
```

**Standard Durations** (stored values):
- 1, 2, 3, 5, 10, 15, 20, 30 seconds
- 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120 minutes
- Extended: 180, 240, 300 minutes (3-5 hours)

---

### 2. CriticalPowerModel

Stores the user's calculated CP and W' values.

```rust
/// Critical Power model parameters
pub struct CriticalPowerModel {
    /// Unique identifier
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Critical Power in watts
    pub cp_watts: u16,
    /// W' (anaerobic work capacity) in joules
    pub w_prime_joules: u32,
    /// When this model was calculated
    pub calculated_at: DateTime<Utc>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// PDC points used for fitting (as JSON)
    pub fitting_points_json: String,
    /// Model is active (most recent)
    pub is_current: bool,
}
```

**SQL Schema**:
```sql
CREATE TABLE IF NOT EXISTS critical_power_models (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    cp_watts INTEGER NOT NULL,
    w_prime_joules INTEGER NOT NULL,
    calculated_at TEXT NOT NULL,
    confidence REAL NOT NULL,
    fitting_points_json TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_cp_user_id ON critical_power_models(user_id);
CREATE INDEX IF NOT EXISTS idx_cp_current ON critical_power_models(user_id, is_current);
```

---

### 3. FtpEstimate

Stores auto-detected FTP values.

```rust
/// FTP auto-detection result
pub struct FtpEstimate {
    /// Unique identifier
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Estimated FTP in watts
    pub ftp_watts: u16,
    /// Detection method used
    pub detection_method: FtpDetectionMethod,
    /// Confidence level
    pub confidence: FtpConfidence,
    /// When estimated
    pub estimated_at: DateTime<Utc>,
    /// User accepted this estimate
    pub is_accepted: bool,
    /// When accepted (if applicable)
    pub accepted_at: Option<DateTime<Utc>>,
}

pub enum FtpDetectionMethod {
    /// 95% of 20-minute power
    TwentyMinute,
    /// Average of 45-60 minute efforts
    ExtendedDuration,
    /// Derived from CP model
    CriticalPower,
}

pub enum FtpConfidence {
    High,    // 3+ recent efforts
    Medium,  // 2+ efforts
    Low,     // Limited data
}
```

**SQL Schema**:
```sql
CREATE TABLE IF NOT EXISTS ftp_estimates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    ftp_watts INTEGER NOT NULL,
    detection_method TEXT NOT NULL,
    confidence TEXT NOT NULL,
    estimated_at TEXT NOT NULL,
    is_accepted INTEGER NOT NULL DEFAULT 0,
    accepted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_ftp_user_id ON ftp_estimates(user_id);
CREATE INDEX IF NOT EXISTS idx_ftp_accepted ON ftp_estimates(user_id, is_accepted);
```

---

### 4. DailyTrainingLoad

Stores daily aggregated training metrics.

```rust
/// Daily training load summary
pub struct DailyTrainingLoad {
    /// User ID
    pub user_id: String,
    /// Date (YYYY-MM-DD)
    pub date: NaiveDate,
    /// Total TSS for the day
    pub tss: f32,
    /// Acute Training Load (rolling 7-day EWMA)
    pub atl: f32,
    /// Chronic Training Load (rolling 42-day EWMA)
    pub ctl: f32,
    /// Training Stress Balance (CTL - ATL)
    pub tsb: f32,
    /// Number of rides
    pub ride_count: u32,
    /// Total duration in seconds
    pub total_duration_seconds: u32,
}
```

**SQL Schema**:
```sql
CREATE TABLE IF NOT EXISTS daily_training_load (
    user_id TEXT NOT NULL REFERENCES users(id),
    date TEXT NOT NULL,
    tss REAL NOT NULL,
    atl REAL NOT NULL,
    ctl REAL NOT NULL,
    tsb REAL NOT NULL,
    ride_count INTEGER NOT NULL,
    total_duration_seconds INTEGER NOT NULL,
    PRIMARY KEY (user_id, date)
);

CREATE INDEX IF NOT EXISTS idx_dtl_user_date ON daily_training_load(user_id, date);
```

---

### 5. Vo2maxEstimate

Stores VO2max calculations over time.

```rust
/// VO2max estimation result
pub struct Vo2maxEstimate {
    /// Unique identifier
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Estimated VO2max in ml/kg/min
    pub vo2max: f32,
    /// 5-minute power used for calculation
    pub power_5min: u16,
    /// Body weight used (kg)
    pub weight_kg: f32,
    /// Percentile vs population (0-100)
    pub percentile: u8,
    /// When calculated
    pub calculated_at: DateTime<Utc>,
}
```

**SQL Schema**:
```sql
CREATE TABLE IF NOT EXISTS vo2max_estimates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    vo2max REAL NOT NULL,
    power_5min INTEGER NOT NULL,
    weight_kg REAL NOT NULL,
    percentile INTEGER NOT NULL,
    calculated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vo2_user_id ON vo2max_estimates(user_id);
```

---

### 6. RiderProfile (Extended)

Extends existing user profile with analytics-specific fields.

```rust
/// Rider classification and analytics profile
pub struct RiderAnalyticsProfile {
    /// User ID
    pub user_id: String,
    /// Rider type classification
    pub rider_type: Option<RiderType>,
    /// When classification was calculated
    pub classified_at: Option<DateTime<Utc>>,
    /// Power profile strengths (normalized 0-1)
    pub neuromuscular_score: Option<f32>,
    pub anaerobic_score: Option<f32>,
    pub vo2max_score: Option<f32>,
    pub threshold_score: Option<f32>,
}

pub enum RiderType {
    Sprinter,
    Pursuiter,
    Climber,
    TimeTrialist,
    AllRounder,
}
```

**SQL Schema**:
```sql
CREATE TABLE IF NOT EXISTS rider_analytics_profile (
    user_id TEXT PRIMARY KEY REFERENCES users(id),
    rider_type TEXT,
    classified_at TEXT,
    neuromuscular_score REAL,
    anaerobic_score REAL,
    vo2max_score REAL,
    threshold_score REAL
);
```

---

## Entity Relationships

```
┌─────────────┐      ┌───────────────────────┐
│   users     │──1:N──│  power_duration_curve │
└─────────────┘      └───────────────────────┘
       │                        │
       │                        │
       │              ┌─────────┴─────────┐
       │              │      rides        │
       │              └───────────────────┘
       │
       ├──1:N──┬──────────────────────────────┐
       │       │                               │
       │  ┌────┴───────────────────┐    ┌─────┴─────────────┐
       │  │ critical_power_models  │    │  ftp_estimates    │
       │  └────────────────────────┘    └───────────────────┘
       │
       ├──1:N──────────────────────────────────┐
       │                                        │
       │  ┌────────────────────────┐    ┌──────┴──────────────┐
       │  │  daily_training_load   │    │  vo2max_estimates   │
       │  └────────────────────────┘    └─────────────────────┘
       │
       └──1:1──────────────────────────────────┐
                                               │
                                   ┌───────────┴───────────┐
                                   │ rider_analytics_profile│
                                   └───────────────────────┘
```

---

## State Transitions

### FTP Estimate Lifecycle

```
┌─────────┐     detect()     ┌───────────┐     accept()     ┌──────────┐
│ (none)  │ ───────────────> │ Pending   │ ───────────────> │ Accepted │
└─────────┘                  └───────────┘                  └──────────┘
                                   │                              │
                                   │ reject()                     │
                                   v                              │
                              ┌─────────┐                         │
                              │ Ignored │                         │
                              └─────────┘                         │
                                                                  │
     new detection triggers re-evaluation <───────────────────────┘
```

### PDC Update Lifecycle

```
┌─────────────┐   new ride   ┌───────────────┐   analyze   ┌────────────┐
│ PDC (empty) │ ───────────> │ Extract MMP   │ ──────────> │ Update PDC │
└─────────────┘              └───────────────┘             └────────────┘
                                                                  │
                                                                  v
     ┌─────────────────────────────────────────────────────────────────────┐
     │ For each duration where new_power > stored_power:                   │
     │   - Update power_duration_curve                                      │
     │   - Trigger CP/W' recalculation if 2-20 min range affected          │
     │   - Trigger VO2max recalculation if 5-min power affected            │
     │   - Trigger rider classification if any key duration affected       │
     └─────────────────────────────────────────────────────────────────────┘
```

---

## Validation Rules

### Power Duration Curve
- `duration_seconds` must be > 0
- `max_power` must be > 0 and ≤ 2500 (configurable)
- `achieved_at` must be ≤ current time
- Only one entry per (user_id, duration_seconds)

### Critical Power Model
- `cp_watts` must be between 50-500 (reasonable range)
- `w_prime_joules` must be between 5000-50000 (5-50 kJ)
- `confidence` must be between 0.0-1.0
- `fitting_points_json` must contain valid JSON array

### FTP Estimate
- `ftp_watts` must be between 50-500
- `confidence` must be one of: High, Medium, Low
- Only one `is_accepted = true` per user at a time

### Daily Training Load
- `tss` must be ≥ 0
- `atl`, `ctl` must be ≥ 0
- `date` must be valid ISO date
- No duplicate (user_id, date) entries

### VO2max Estimate
- `vo2max` must be between 20-90 (ml/kg/min realistic range)
- `power_5min` must be > 0
- `weight_kg` must be between 30-200
- `percentile` must be between 0-100

---

## Migration Strategy

### Schema Version Increment
Current: 1 → New: 2

### Migration SQL
```sql
-- Migration 2: Add analytics tables
CREATE TABLE IF NOT EXISTS power_duration_curve (...);
CREATE TABLE IF NOT EXISTS critical_power_models (...);
CREATE TABLE IF NOT EXISTS ftp_estimates (...);
CREATE TABLE IF NOT EXISTS daily_training_load (...);
CREATE TABLE IF NOT EXISTS vo2max_estimates (...);
CREATE TABLE IF NOT EXISTS rider_analytics_profile (...);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_pdc_user_id ON power_duration_curve(user_id);
-- ... additional indexes

-- Update schema version
INSERT INTO schema_version (version, applied_at) VALUES (2, datetime('now'));
```

### Backward Compatibility
- New tables only - no changes to existing tables
- Existing data preserved
- Analytics calculated on-demand from ride_samples
