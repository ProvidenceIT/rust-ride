# Cloud API Contract

**Feature Branch**: `004-ai-ml-coaching`
**Version**: 1.0.0
**Base URL**: `https://api.rustride.io/v1`

## Overview

RESTful API for ML inference services. All endpoints require authentication via API key.

---

## Authentication

```
Authorization: Bearer <api_key>
```

API keys are issued per-user and stored locally in the RustRide client.

---

## Common Response Format

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2025-01-17T14:30:00Z"
}
```

Error response:
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "INSUFFICIENT_DATA",
    "message": "At least 5 rides required for FTP prediction"
  },
  "timestamp": "2025-01-17T14:30:00Z"
}
```

---

## Endpoints

### POST /predictions/ftp

Generate FTP prediction from ride history.

**Request**:
```json
{
  "user_id": "uuid",
  "rides_history": [
    {
      "ride_id": "uuid",
      "date": "2025-01-15",
      "duration_seconds": 3600,
      "avg_power": 220,
      "normalized_power": 235,
      "max_power": 450,
      "tss": 75,
      "pdc_points": [
        {"duration_secs": 5, "power_watts": 450},
        {"duration_secs": 60, "power_watts": 350},
        {"duration_secs": 300, "power_watts": 280},
        {"duration_secs": 1200, "power_watts": 255},
        {"duration_secs": 3600, "power_watts": 235}
      ]
    }
  ],
  "current_ftp": 250,
  "preferred_method": "auto"
}
```

**Response**:
```json
{
  "ftp_predicted": 255,
  "confidence": "high",
  "method_used": "extended_duration",
  "supporting_efforts": [
    {"duration": 3600, "power": 240, "ride_date": "2025-01-15", "ride_id": "uuid"}
  ],
  "confidence_interval": {"low": 245, "high": 265},
  "differs_from_current": true,
  "difference_percent": 2.0
}
```

**Response Time**: < 5 seconds

---

### POST /predictions/fatigue

Analyze current fatigue state during active ride.

**Request**:
```json
{
  "user_id": "uuid",
  "ride_id": "uuid",
  "samples": [
    {
      "elapsed_seconds": 1800,
      "power_watts": 250,
      "heart_rate_bpm": 155,
      "cadence_rpm": 90
    }
  ],
  "target_power": 250,
  "athlete_baseline": {
    "resting_hr": 55,
    "max_hr": 185,
    "typical_aerobic_decoupling": 0.08,
    "typical_power_variability": 0.12
  }
}
```

**Response**:
```json
{
  "fatigue_indicators": {
    "aerobic_decoupling_score": 0.12,
    "power_variability_index": 1.42,
    "hrv_fatigue_indicator": null
  },
  "alert_triggered": true,
  "severity": "moderate",
  "recommended_action": "consider_recovery",
  "message": "Heart rate drift detected - consider reducing intensity",
  "confidence": 0.87
}
```

**Response Time**: < 2 seconds

---

### POST /recommendations/workouts

Get personalized workout recommendations.

**Request**:
```json
{
  "user_id": "uuid",
  "training_goals": [
    {
      "id": "uuid",
      "type": "improve_vo2max",
      "target_date": null,
      "priority": 1
    }
  ],
  "current_ctl": 65,
  "current_atl": 72,
  "acwr": 1.11,
  "available_minutes": 60,
  "recently_completed": ["workout-id-1", "workout-id-2"],
  "energy_system_history": {
    "vo2max_days_since": 8,
    "threshold_days_since": 3,
    "endurance_days_since": 1
  }
}
```

**Response**:
```json
{
  "recommendations": [
    {
      "workout_id": "vo2max_5x4min",
      "source": "builtin",
      "title": "VO2max: 5×4min @ 120% FTP",
      "suitability_score": 0.92,
      "reasoning": "Targets your VO2max goal; ACWR is optimal for a hard effort",
      "expected_tss": 65,
      "duration_minutes": 55,
      "energy_systems": ["vo2max", "threshold"],
      "difficulty": 7.2
    },
    {
      "workout_id": "vo2max_3x5min",
      "source": "builtin",
      "title": "VO2max: 3×5min @ 115% FTP",
      "suitability_score": 0.85,
      "reasoning": "Slightly lower volume VO2max option",
      "expected_tss": 55,
      "duration_minutes": 50,
      "energy_systems": ["vo2max"],
      "difficulty": 6.8
    }
  ],
  "training_gap": "You haven't done VO2max efforts in 8 days",
  "load_status": "optimal"
}
```

**Response Time**: < 3 seconds

---

### POST /forecasts/ctl

Generate CTL/fitness trend forecast.

**Request**:
```json
{
  "user_id": "uuid",
  "ctl_history": [
    {"date": "2024-12-01", "ctl": 55, "atl": 60, "tss": 65},
    {"date": "2024-12-02", "ctl": 55.5, "atl": 62, "tss": 70}
  ],
  "target_event": {
    "goal_id": "uuid",
    "date": "2025-04-15",
    "target_ctl": 80
  },
  "forecast_weeks": 12
}
```

**Response**:
```json
{
  "forecast": [
    {"date": "2025-01-24", "projected_ctl": 65, "confidence_low": 60, "confidence_high": 70},
    {"date": "2025-02-07", "projected_ctl": 70, "confidence_low": 64, "confidence_high": 76},
    {"date": "2025-02-21", "projected_ctl": 74, "confidence_low": 67, "confidence_high": 81}
  ],
  "trend": "improving",
  "slope": 0.8,
  "plateau_detected": false,
  "detraining_risk": "none",
  "event_readiness": {
    "goal_id": "uuid",
    "target_ctl": 80,
    "projected_ctl_at_event": 76,
    "gap": -4,
    "on_track": false,
    "recommendation": "Increase weekly TSS by 8% to reach target"
  }
}
```

**Response Time**: < 3 seconds

---

### POST /analysis/cadence

Analyze optimal cadence from ride history.

**Request**:
```json
{
  "user_id": "uuid",
  "ride_samples": [
    {
      "ride_id": "uuid",
      "samples": [
        {"power_watts": 200, "cadence_rpm": 85, "elapsed_seconds": 60},
        {"power_watts": 250, "cadence_rpm": 92, "elapsed_seconds": 120}
      ]
    }
  ]
}
```

**Response**:
```json
{
  "optimal_range": {"min": 85, "max": 95},
  "efficiency_by_band": [
    {"band": "70-79", "efficiency": 0.72, "sample_count": 1200},
    {"band": "80-89", "efficiency": 0.88, "sample_count": 4500},
    {"band": "90-99", "efficiency": 0.91, "sample_count": 3200},
    {"band": "100+", "efficiency": 0.78, "sample_count": 800}
  ],
  "degradation_pattern": {
    "detected": true,
    "onset_minutes": 45,
    "variability_increase_percent": 35
  },
  "recommendation": "Your most efficient cadence is 90-95 RPM. Consider focusing on this range for sustained efforts."
}
```

**Response Time**: < 5 seconds

---

### GET /health

Health check endpoint.

**Response**:
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "model_versions": {
    "ftp_model": "2025-01-15",
    "fatigue_model": "2025-01-10",
    "recommendation_model": "2025-01-12"
  }
}
```

---

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| INVALID_REQUEST | 400 | Malformed request body |
| UNAUTHORIZED | 401 | Invalid or missing API key |
| INSUFFICIENT_DATA | 422 | Not enough ride data for prediction |
| RATE_LIMITED | 429 | Too many requests |
| SERVER_ERROR | 500 | Internal server error |
| MODEL_UNAVAILABLE | 503 | ML model temporarily unavailable |

---

## Rate Limits

| Plan | Requests/day | Burst |
|------|-------------|-------|
| Free | 50 | 10/min |
| Pro | 500 | 60/min |

---

## Offline Handling

Client should:
1. Queue requests when 5xx errors or network unavailable
2. Retry with exponential backoff (30s, 60s, 2m, 5m)
3. Max queue size: 50 requests
4. Display cached predictions with "last updated" timestamp
