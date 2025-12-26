# Data Model: Social & Multiplayer

**Feature Branch**: `006-social-multiplayer`
**Date**: 2025-12-26

## Entity Relationship Diagram

```
┌─────────────┐       ┌─────────────────┐       ┌─────────────┐
│   Rider     │──────<│  ClubMembership │>──────│    Club     │
└─────────────┘       └─────────────────┘       └─────────────┘
      │                                               │
      │ 1:N                                          │ 1:N
      ▼                                               ▼
┌─────────────┐                              ┌───────────────┐
│ RiderBadge  │                              │ ClubChallenge │
└─────────────┘                              └───────────────┘
      │
      │ N:1
      ▼
┌─────────────┐
│   Badge     │
└─────────────┘

┌─────────────┐       ┌─────────────────┐       ┌─────────────┐
│   Rider     │──────<│  SegmentEffort  │>──────│   Segment   │
└─────────────┘       └─────────────────┘       └─────────────┘

┌─────────────┐       ┌─────────────────┐       ┌─────────────┐
│   Rider     │──────<│ ChallengeProgress│>─────│  Challenge  │
└─────────────┘       └─────────────────┘       └─────────────┘

┌─────────────┐       ┌─────────────────┐       ┌─────────────┐
│   Rider     │──────<│  WorkoutRating  │>──────│   Workout   │
└─────────────┘       └─────────────────┘       └─────────────┘

┌─────────────┐       ┌─────────────────┐       ┌─────────────┐
│   Rider     │──────<│ RaceParticipant │>──────│  RaceEvent  │
└─────────────┘       └─────────────────┘       └─────────────┘

┌─────────────┐       ┌─────────────────┐
│ GroupRide   │──────<│  ChatMessage    │
└─────────────┘       └─────────────────┘
      │
      │ N:M
      ▼
┌─────────────┐
│   Rider     │
└─────────────┘
```

## Entities

### Rider (extends existing user concept)

Represents an individual user with social features.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Unique rider identifier |
| `display_name` | String | NOT NULL, max 50 chars | Public display name |
| `avatar_id` | String | Optional | Reference to avatar asset |
| `bio` | String | Optional, max 500 chars | Short biography |
| `ftp` | u16 | Optional | Current FTP for context |
| `total_distance_km` | f64 | DEFAULT 0 | Lifetime distance |
| `total_time_hours` | f64 | DEFAULT 0 | Lifetime ride time |
| `sharing_enabled` | bool | DEFAULT true | Allow activity sharing on LAN |
| `created_at` | DateTime | NOT NULL | Profile creation time |
| `updated_at` | DateTime | NOT NULL | Last profile update |

**Validation Rules**:
- `display_name` must be non-empty and unique locally
- `avatar_id` references built-in avatar assets

### Club

Organization of riders with aggregate statistics.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Unique club identifier |
| `name` | String | NOT NULL, max 100 chars | Club name |
| `description` | String | Optional, max 500 chars | Club description |
| `join_code` | String | UNIQUE, 8 chars | Code for joining |
| `admin_rider_id` | UUID | FK → Rider | Club administrator |
| `total_distance_km` | f64 | DEFAULT 0 | Aggregate member distance |
| `total_time_hours` | f64 | DEFAULT 0 | Aggregate member time |
| `created_at` | DateTime | NOT NULL | Club creation time |

**Validation Rules**:
- `join_code` auto-generated, alphanumeric
- Only admin can delete club or transfer ownership

### ClubMembership

Association between riders and clubs.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Membership ID |
| `club_id` | UUID | FK → Club | Club reference |
| `rider_id` | UUID | FK → Rider | Rider reference |
| `joined_at` | DateTime | NOT NULL | Join timestamp |
| `left_at` | DateTime | Optional | Leave timestamp (null = active) |

**State Transitions**:
- ACTIVE: `left_at` is NULL
- INACTIVE: `left_at` is set

### Badge

Achievement milestone definitions.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | String | PK | Badge identifier (e.g., "century_rider") |
| `name` | String | NOT NULL | Display name |
| `description` | String | NOT NULL | How to earn it |
| `icon` | String | NOT NULL | Icon asset reference |
| `category` | BadgeCategory | NOT NULL | Distance, FTP, Consistency, Special |
| `criteria_type` | CriteriaType | NOT NULL | Type of criteria to check |
| `criteria_value` | f64 | NOT NULL | Threshold value |

**Badge Categories** (enum):
- `Distance`: Total distance milestones
- `FTP`: FTP improvement milestones
- `Consistency`: Streak-based achievements
- `Special`: Event or unique achievements

**Criteria Types** (enum):
- `TotalDistanceKm`: criteria_value = km threshold
- `FtpIncrease`: criteria_value = watts improvement
- `ConsecutiveDays`: criteria_value = days count
- `WorkoutsCompleted`: criteria_value = workout count

### RiderBadge

Badges earned by riders.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Record ID |
| `rider_id` | UUID | FK → Rider | Owner |
| `badge_id` | String | FK → Badge | Badge earned |
| `unlocked_at` | DateTime | NOT NULL | When earned |

**Validation Rules**:
- Unique constraint on (rider_id, badge_id)

### Segment

Pre-defined route section for leaderboards.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Segment identifier |
| `world_id` | String | NOT NULL | Which world/route |
| `name` | String | NOT NULL | Segment name |
| `start_distance_m` | f64 | NOT NULL | Start point on route |
| `end_distance_m` | f64 | NOT NULL | End point on route |
| `category` | SegmentCategory | NOT NULL | Climb, Sprint, Mixed |
| `elevation_gain_m` | f64 | DEFAULT 0 | Total climbing |

**Segment Categories** (enum):
- `Climb`: Primarily uphill
- `Sprint`: Flat or downhill, short
- `Mixed`: General segment

**Validation Rules**:
- `end_distance_m` > `start_distance_m`
- Bundled with world content (not user-editable)

### SegmentEffort

Individual attempt at a segment.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Effort identifier |
| `segment_id` | UUID | FK → Segment | Which segment |
| `rider_id` | UUID | FK → Rider | Who rode it |
| `ride_id` | UUID | FK → Ride | Parent ride record |
| `elapsed_time_ms` | u32 | NOT NULL | Time to complete |
| `avg_power_watts` | u16 | Optional | Average power |
| `avg_hr_bpm` | u8 | Optional | Average heart rate |
| `recorded_at` | DateTime | NOT NULL | When completed |
| `imported` | bool | DEFAULT false | From external source |
| `import_source_name` | String | Optional | Original rider name if imported |

**Validation Rules**:
- Unique constraint on (segment_id, rider_id, recorded_at) for deduplication

### Challenge

Training goal definition.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Challenge identifier |
| `name` | String | NOT NULL, max 100 chars | Challenge name |
| `description` | String | Optional | Challenge details |
| `goal_type` | GoalType | NOT NULL | Type of goal |
| `goal_value` | f64 | NOT NULL | Target value |
| `duration_days` | u16 | NOT NULL | Challenge duration |
| `start_date` | Date | NOT NULL | When it starts |
| `end_date` | Date | NOT NULL | When it ends |
| `created_by_rider_id` | UUID | FK → Rider | Creator |
| `created_at` | DateTime | NOT NULL | Creation time |

**Goal Types** (enum):
- `TotalDistanceKm`: Ride X km total
- `TotalTimeHours`: Ride X hours total
- `TotalTss`: Accumulate X TSS
- `WorkoutCount`: Complete X workouts
- `WorkoutTypeCount`: Complete X workouts of specific type

**Validation Rules**:
- `end_date` > `start_date`
- `goal_value` > 0

### ChallengeProgress

Rider's progress on a challenge.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Progress record ID |
| `challenge_id` | UUID | FK → Challenge | Which challenge |
| `rider_id` | UUID | FK → Rider | Who is tracking |
| `current_value` | f64 | DEFAULT 0 | Progress toward goal |
| `completed` | bool | DEFAULT false | Goal achieved |
| `completed_at` | DateTime | Optional | When completed |
| `last_updated` | DateTime | NOT NULL | Last progress update |

### WorkoutRating

User ratings and reviews for workouts.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Rating ID |
| `workout_id` | String | FK → Workout | Which workout |
| `rider_id` | UUID | FK → Rider | Who rated |
| `rating` | u8 | 1-5 | Star rating |
| `review_text` | String | Optional, max 1000 chars | Written review |
| `created_at` | DateTime | NOT NULL | When rated |

**Validation Rules**:
- `rating` must be 1, 2, 3, 4, or 5
- Unique constraint on (workout_id, rider_id)

### RaceEvent

Scheduled virtual race.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Race identifier |
| `name` | String | NOT NULL | Race name |
| `world_id` | String | NOT NULL | Which world |
| `route_id` | String | NOT NULL | Which route |
| `distance_km` | f64 | NOT NULL | Race distance |
| `scheduled_start` | DateTime | NOT NULL | When race starts |
| `status` | RaceStatus | NOT NULL | Current state |
| `organizer_rider_id` | UUID | FK → Rider | Who created it |
| `created_at` | DateTime | NOT NULL | Creation time |

**Race Status** (enum):
- `Scheduled`: Waiting for start time
- `Countdown`: 60 second countdown active
- `InProgress`: Race running
- `Finished`: Race completed
- `Cancelled`: Race cancelled

### RaceParticipant

Rider participation in a race.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Participant record |
| `race_id` | UUID | FK → RaceEvent | Which race |
| `rider_id` | UUID | FK → Rider | Which rider |
| `status` | ParticipantStatus | NOT NULL | Current state |
| `finish_time_ms` | u32 | Optional | Time if finished |
| `finish_position` | u16 | Optional | Rank if finished |
| `joined_at` | DateTime | NOT NULL | When joined |
| `disconnected_at` | DateTime | Optional | When disconnected |

**Participant Status** (enum):
- `Registered`: Joined but race not started
- `Racing`: Currently racing
- `Finished`: Completed the race
- `DNF`: Did Not Finish (disconnected >60s)

### GroupRide

Active LAN group ride session (in-memory + persisted summary).

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Session identifier |
| `host_rider_id` | UUID | FK → Rider | Who started it |
| `name` | String | Optional | Session name |
| `world_id` | String | NOT NULL | Which world |
| `started_at` | DateTime | NOT NULL | Session start |
| `ended_at` | DateTime | Optional | Session end |
| `max_participants` | u8 | DEFAULT 10 | Limit |

### GroupRideParticipant

Riders in a group ride (mostly in-memory).

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Participation ID |
| `group_ride_id` | UUID | FK → GroupRide | Which session |
| `rider_id` | UUID | FK → Rider | Which rider |
| `joined_at` | DateTime | NOT NULL | When joined |
| `left_at` | DateTime | Optional | When left |

### ChatMessage

Messages during group rides.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Message ID |
| `group_ride_id` | UUID | FK → GroupRide | Which session |
| `sender_rider_id` | UUID | FK → Rider | Who sent it |
| `message_text` | String | NOT NULL, max 500 chars | Message content |
| `sent_at` | DateTime | NOT NULL | When sent |

### ActivitySummary

Shareable ride summary.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK | Summary ID |
| `ride_id` | UUID | FK → Ride | Source ride |
| `rider_id` | UUID | FK → Rider | Rider |
| `rider_name` | String | NOT NULL | Name at time of ride |
| `distance_km` | f64 | NOT NULL | Ride distance |
| `duration_minutes` | u32 | NOT NULL | Ride duration |
| `avg_power_watts` | u16 | Optional | Average power |
| `elevation_gain_m` | f64 | DEFAULT 0 | Climbing |
| `world_id` | String | Optional | Where ridden |
| `recorded_at` | DateTime | NOT NULL | Ride date |
| `shared` | bool | DEFAULT true | Visible to LAN |

## SQLite Schema Extensions

```sql
-- New tables for Social & Multiplayer feature

-- Rider profile (extends or replaces simple user settings)
CREATE TABLE IF NOT EXISTS riders (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    avatar_id TEXT,
    bio TEXT,
    ftp INTEGER,
    total_distance_km REAL DEFAULT 0,
    total_time_hours REAL DEFAULT 0,
    sharing_enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Clubs
CREATE TABLE IF NOT EXISTS clubs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    join_code TEXT UNIQUE NOT NULL,
    admin_rider_id TEXT NOT NULL REFERENCES riders(id),
    total_distance_km REAL DEFAULT 0,
    total_time_hours REAL DEFAULT 0,
    created_at TEXT NOT NULL
);

-- Club memberships
CREATE TABLE IF NOT EXISTS club_memberships (
    id TEXT PRIMARY KEY,
    club_id TEXT NOT NULL REFERENCES clubs(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    joined_at TEXT NOT NULL,
    left_at TEXT
);

-- Badges (seeded on first run)
CREATE TABLE IF NOT EXISTS badges (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    icon TEXT NOT NULL,
    category TEXT NOT NULL,
    criteria_type TEXT NOT NULL,
    criteria_value REAL NOT NULL
);

-- Earned badges
CREATE TABLE IF NOT EXISTS rider_badges (
    id TEXT PRIMARY KEY,
    rider_id TEXT NOT NULL REFERENCES riders(id),
    badge_id TEXT NOT NULL REFERENCES badges(id),
    unlocked_at TEXT NOT NULL,
    UNIQUE(rider_id, badge_id)
);

-- Segments (seeded from world data)
CREATE TABLE IF NOT EXISTS segments (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    name TEXT NOT NULL,
    start_distance_m REAL NOT NULL,
    end_distance_m REAL NOT NULL,
    category TEXT NOT NULL,
    elevation_gain_m REAL DEFAULT 0
);

-- Segment efforts
CREATE TABLE IF NOT EXISTS segment_efforts (
    id TEXT PRIMARY KEY,
    segment_id TEXT NOT NULL REFERENCES segments(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    ride_id TEXT,
    elapsed_time_ms INTEGER NOT NULL,
    avg_power_watts INTEGER,
    avg_hr_bpm INTEGER,
    recorded_at TEXT NOT NULL,
    imported INTEGER DEFAULT 0,
    import_source_name TEXT
);

-- Challenges
CREATE TABLE IF NOT EXISTS challenges (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    goal_type TEXT NOT NULL,
    goal_value REAL NOT NULL,
    duration_days INTEGER NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    created_by_rider_id TEXT REFERENCES riders(id),
    created_at TEXT NOT NULL
);

-- Challenge progress
CREATE TABLE IF NOT EXISTS challenge_progress (
    id TEXT PRIMARY KEY,
    challenge_id TEXT NOT NULL REFERENCES challenges(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    current_value REAL DEFAULT 0,
    completed INTEGER DEFAULT 0,
    completed_at TEXT,
    last_updated TEXT NOT NULL,
    UNIQUE(challenge_id, rider_id)
);

-- Workout ratings
CREATE TABLE IF NOT EXISTS workout_ratings (
    id TEXT PRIMARY KEY,
    workout_id TEXT NOT NULL,
    rider_id TEXT NOT NULL REFERENCES riders(id),
    rating INTEGER NOT NULL CHECK(rating >= 1 AND rating <= 5),
    review_text TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(workout_id, rider_id)
);

-- Race events
CREATE TABLE IF NOT EXISTS race_events (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    world_id TEXT NOT NULL,
    route_id TEXT NOT NULL,
    distance_km REAL NOT NULL,
    scheduled_start TEXT NOT NULL,
    status TEXT NOT NULL,
    organizer_rider_id TEXT NOT NULL REFERENCES riders(id),
    created_at TEXT NOT NULL
);

-- Race participants
CREATE TABLE IF NOT EXISTS race_participants (
    id TEXT PRIMARY KEY,
    race_id TEXT NOT NULL REFERENCES race_events(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    status TEXT NOT NULL,
    finish_time_ms INTEGER,
    finish_position INTEGER,
    joined_at TEXT NOT NULL,
    disconnected_at TEXT
);

-- Group rides
CREATE TABLE IF NOT EXISTS group_rides (
    id TEXT PRIMARY KEY,
    host_rider_id TEXT NOT NULL REFERENCES riders(id),
    name TEXT,
    world_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    max_participants INTEGER DEFAULT 10
);

-- Group ride participants
CREATE TABLE IF NOT EXISTS group_ride_participants (
    id TEXT PRIMARY KEY,
    group_ride_id TEXT NOT NULL REFERENCES group_rides(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    joined_at TEXT NOT NULL,
    left_at TEXT
);

-- Chat messages
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    group_ride_id TEXT NOT NULL REFERENCES group_rides(id),
    sender_rider_id TEXT NOT NULL REFERENCES riders(id),
    message_text TEXT NOT NULL,
    sent_at TEXT NOT NULL
);

-- Activity summaries
CREATE TABLE IF NOT EXISTS activity_summaries (
    id TEXT PRIMARY KEY,
    ride_id TEXT,
    rider_id TEXT NOT NULL REFERENCES riders(id),
    rider_name TEXT NOT NULL,
    distance_km REAL NOT NULL,
    duration_minutes INTEGER NOT NULL,
    avg_power_watts INTEGER,
    elevation_gain_m REAL DEFAULT 0,
    world_id TEXT,
    recorded_at TEXT NOT NULL,
    shared INTEGER DEFAULT 1
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_segment_efforts_segment ON segment_efforts(segment_id);
CREATE INDEX IF NOT EXISTS idx_segment_efforts_rider ON segment_efforts(rider_id);
CREATE INDEX IF NOT EXISTS idx_challenge_progress_challenge ON challenge_progress(challenge_id);
CREATE INDEX IF NOT EXISTS idx_workout_ratings_workout ON workout_ratings(workout_id);
CREATE INDEX IF NOT EXISTS idx_activity_summaries_rider ON activity_summaries(rider_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_group_ride ON chat_messages(group_ride_id);
```

## File Formats

### Challenge Export (TOML)

```toml
[challenge]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "100km Week"
description = "Ride 100 kilometers this week"
goal_type = "TotalDistanceKm"
goal_value = 100.0
duration_days = 7
start_date = "2025-01-01"
end_date = "2025-01-07"
created_by = "John Doe"
```

### Leaderboard Export (JSON)

```json
{
  "segment": {
    "id": "seg-123",
    "name": "Epic KOM",
    "world_id": "mountains",
    "distance_m": 5000,
    "elevation_gain_m": 350
  },
  "efforts": [
    {
      "rank": 1,
      "rider_name": "Jane Rider",
      "elapsed_time_ms": 720000,
      "avg_power_watts": 285,
      "recorded_at": "2025-01-15T14:30:00Z"
    }
  ],
  "exported_at": "2025-01-20T10:00:00Z",
  "export_version": "1.0"
}
```

### Activity Summary (JSON for LAN sharing)

```json
{
  "id": "act-123",
  "rider": {
    "id": "rider-456",
    "name": "John Doe",
    "avatar_id": "avatar_01"
  },
  "distance_km": 45.5,
  "duration_minutes": 90,
  "avg_power_watts": 210,
  "elevation_gain_m": 650,
  "world_id": "coastal",
  "recorded_at": "2025-01-20T08:00:00Z"
}
```
