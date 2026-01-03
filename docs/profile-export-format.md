# RustRide Profile Export Format

This document describes the JSON export format for rider profiles in RustRide. The export format enables profile backup, transfer between installations, and data portability.

## Overview

Profile exports contain a complete snapshot of a rider's profile data including:

- Core profile information (display name, bio, stats)
- FTP history records
- Avatar customization settings

## Current Version

**Export Format Version:** `1.0`

All exports include a version identifier to ensure compatibility during import operations.

---

## JSON Schema

### Root Structure

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `export_version` | string | Yes | Format version (e.g., "1.0") |
| `exported_at` | string (ISO 8601) | Yes | Export timestamp in RFC 3339 format |
| `rider_id` | string (UUID) | Yes | Unique rider identifier |
| `profile` | object | Yes | Core profile data |
| `ftp_history` | array | Yes | FTP test history records |
| `avatar` | object \| null | No | Avatar customization (null if not configured) |

### ProfileData Object

The `profile` field contains core rider information:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `display_name` | string | Yes | Display name visible to other riders (1-50 chars) |
| `bio` | string \| null | No | Optional bio/description text |
| `ftp` | integer \| null | No | Current FTP in watts (100-600 typical range) |
| `total_distance_km` | number | Yes | Cumulative distance in kilometers |
| `total_time_hours` | number | Yes | Cumulative ride time in hours |
| `sharing_enabled` | boolean | Yes | Whether profile is visible to other riders |

### FtpHistoryEntry Object

Each entry in `ftp_history` represents an FTP test or estimate:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ftp_watts` | integer | Yes | FTP value in watts (u16: 0-65535) |
| `method` | string | Yes | Detection method identifier |
| `confidence` | string | Yes | Confidence level of the estimate |
| `detected_at` | string (ISO 8601) | Yes | When FTP was detected/tested |
| `accepted` | boolean | Yes | Whether user accepted this estimate |

#### FTP Detection Methods

| Value | Description |
|-------|-------------|
| `ramp_test` | Incremental ramp test to exhaustion |
| `20min_test` | 20-minute max effort test (95% of avg) |
| `8min_test` | Dual 8-minute test protocol |
| `manual` | Manually entered by user |
| `ai_detected` | AI-powered detection from ride data |
| `workout_analysis` | Inferred from workout performance |

#### Confidence Levels

| Value | Description |
|-------|-------------|
| `high` | High confidence (dedicated test, consistent power) |
| `medium` | Moderate confidence (good data, some variability) |
| `low` | Low confidence (limited data, estimation) |

### AvatarExport Object

The `avatar` field contains visual customization settings:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `jersey_color` | string | Yes | Primary jersey color as hex (e.g., "#FF5733") |
| `bike_style` | string | Yes | Bike type identifier |
| `jersey_secondary` | string \| null | No | Secondary jersey color as hex |
| `helmet_color` | string \| null | No | Helmet color as hex |

#### Bike Styles

| Value | Description |
|-------|-------------|
| `road_bike` | Standard road racing bike |
| `tt_bike` | Time trial / triathlon bike |
| `gravel` | Gravel/adventure bike |
| `mtb` | Mountain bike |
| `hybrid` | Hybrid/fitness bike |

---

## Example Export

### Complete Profile Export

```json
{
  "export_version": "1.0",
  "exported_at": "2026-01-03T12:00:00Z",
  "rider_id": "550e8400-e29b-41d4-a716-446655440000",
  "profile": {
    "display_name": "PowerRider42",
    "bio": "Weekend warrior. Climbing enthusiast.",
    "ftp": 285,
    "total_distance_km": 4523.7,
    "total_time_hours": 182.5,
    "sharing_enabled": true
  },
  "ftp_history": [
    {
      "ftp_watts": 285,
      "method": "ramp_test",
      "confidence": "high",
      "detected_at": "2026-01-02T10:30:00Z",
      "accepted": true
    },
    {
      "ftp_watts": 275,
      "method": "20min_test",
      "confidence": "high",
      "detected_at": "2025-11-15T08:00:00Z",
      "accepted": true
    },
    {
      "ftp_watts": 260,
      "method": "ai_detected",
      "confidence": "medium",
      "detected_at": "2025-09-01T14:22:00Z",
      "accepted": false
    }
  ],
  "avatar": {
    "jersey_color": "#FF5733",
    "bike_style": "road_bike",
    "jersey_secondary": "#FFFFFF",
    "helmet_color": "#333333"
  }
}
```

### Minimal Profile Export

A profile with no FTP history or avatar configured:

```json
{
  "export_version": "1.0",
  "exported_at": "2026-01-03T09:15:30Z",
  "rider_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "profile": {
    "display_name": "NewRider",
    "bio": null,
    "ftp": null,
    "total_distance_km": 0.0,
    "total_time_hours": 0.0,
    "sharing_enabled": false
  },
  "ftp_history": [],
  "avatar": null
}
```

---

## Version Compatibility

### Current Behavior (v1.0)

- **Import Validation:** Only exact version matches are accepted
- **Version Check:** Import fails with `InvalidVersion` error if versions differ
- **No Migration:** Version 1.0 does not support migration from older formats

### Version Policy

The `export_version` field follows semantic versioning principles:

| Version Change | Compatibility |
|----------------|---------------|
| Patch (1.0.x) | Fully backward compatible |
| Minor (1.x.0) | Backward compatible with optional new fields |
| Major (x.0.0) | Breaking changes, migration logic required |

### Future Compatibility

When newer export versions are released:

1. **Export files remain valid** - Old exports can be migrated to new formats
2. **Migration support** - Future versions may include automatic migration for older formats
3. **Version negotiation** - Import operations will check version and apply appropriate handlers

### Handling Version Mismatches

When importing a file with an incompatible version:

```rust
Err(ProfileExportError::InvalidVersion {
    expected: "1.0",
    found: "2.0"
})
```

**Recommended approach:**
1. Check if export version is newer than current supported version
2. If newer, inform user to update RustRide
3. If older, apply migration logic (when available)

---

## Data Types Reference

### Timestamps

All timestamps use ISO 8601 / RFC 3339 format in UTC:

```
YYYY-MM-DDTHH:MM:SSZ
```

Example: `2026-01-03T12:00:00Z`

### UUIDs

Rider IDs use standard UUID v4 format:

```
xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

Example: `550e8400-e29b-41d4-a716-446655440000`

### Colors

Colors use 6-digit hexadecimal format with `#` prefix:

```
#RRGGBB
```

Examples:
- `#FF5733` - Orange-red
- `#FFFFFF` - White
- `#000000` - Black

### Numeric Precision

| Field | Type | Range |
|-------|------|-------|
| `ftp` | u16 | 0 - 65,535 watts |
| `ftp_watts` | u16 | 0 - 65,535 watts |
| `total_distance_km` | f64 | 0.0 - ~1.8e308 km |
| `total_time_hours` | f64 | 0.0 - ~1.8e308 hours |

---

## Import Behavior

### Conflict Detection

When importing to a database with existing data, these conflicts may be detected:

| Conflict Type | Description |
|---------------|-------------|
| `ExistingProfile` | Profile with same `rider_id` already exists |
| `DisplayNameMismatch` | Imported display name differs from existing |
| `FtpMismatch` | Current FTP value differs between import and existing |
| `AvatarMismatch` | Avatar configuration differs between import and existing |

### Resolution Strategies

| Strategy | Profile | FTP History | Avatar |
|----------|---------|-------------|--------|
| **Replace** | Overwrite | Delete all, import fresh | Overwrite |
| **Merge** | Update | Combine, skip duplicates | Update if different |
| **Skip** | No change | No change | No change |

### FTP History Deduplication

When using the **Merge** strategy, FTP entries are deduplicated by `detected_at` timestamp:

- If an entry with the same `detected_at` exists, it is skipped
- Entries with unique timestamps are added
- Import result tracks `ftp_entries_imported` and `ftp_entries_skipped` counts

---

## File Conventions

### Recommended File Extension

`.json`

### Suggested Naming Pattern

```
rustride-profile-{display_name}-{date}.json
```

Example: `rustride-profile-PowerRider42-2026-01-03.json`

### Character Encoding

UTF-8 encoding is required. The format supports Unicode characters in:
- Display names
- Bio text
- Any string fields

### Pretty Printing

Exports use indented (pretty-printed) JSON for human readability. Imports accept both pretty-printed and minified JSON.

---

## Validation Checklist

Before importing a profile export, verify:

- [ ] `export_version` is "1.0"
- [ ] `exported_at` is valid ISO 8601 timestamp
- [ ] `rider_id` is valid UUID format
- [ ] `profile.display_name` is non-empty string
- [ ] `profile.sharing_enabled` is boolean
- [ ] `ftp_history` is an array (can be empty)
- [ ] All FTP entries have required fields
- [ ] Color values (if present) are valid hex format

---

## Related Documentation

- **Module Documentation:** `src/social/export.rs` - Rust API documentation
- **Database Schema:** `src/storage/schema.rs` - Table definitions
- **Avatar Configuration:** `src/world/avatar.rs` - Avatar types and colors

---

## Changelog

### Version 1.0 (Initial Release)

- Complete profile export with core data, FTP history, and avatar
- Import with conflict detection and resolution strategies
- Support for Merge, Replace, and Skip import strategies
- FTP history deduplication on import
