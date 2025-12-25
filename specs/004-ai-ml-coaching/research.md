# Research: AI & Machine Learning Coaching

**Feature Branch**: `004-ai-ml-coaching`
**Date**: 2025-12-25
**Status**: Complete

## Overview

Research findings for ML approaches in cycling training analytics, covering FTP prediction, fatigue detection, workout recommendations, performance forecasting, and cloud API design.

---

## 1. FTP Prediction Algorithms

**Decision**: Hybrid approach using extended duration power (45-60 min efforts) as primary method, with 95% of 20-minute power as fallback, plus confidence scoring.

**Rationale**:
- RustRide already implements this approach in `ftp_detection.rs` with empirical validation
- Traditional FTP tests often overestimate by 5-10% (Coggan research)
- Using actual sustained efforts from training provides realistic, rider-specific estimates
- Confidence scoring (High/Medium/Low) based on data recency and effort variety prevents false positives

**Algorithm**:
- Extended duration method: Average of 45-min and 60-min PDC values
- Fallback: 20-min × 0.95 when extended data unavailable
- Confidence threshold: High requires 3+ recent quality efforts within 6 weeks
- Change detection: Only notify on >3% variation to reduce alert fatigue

**Alternatives Considered**:

| Approach | Why Rejected |
|----------|-------------|
| Linear regression on all power values | Adds noise from recovery rides; misses high-intensity patterns |
| Neural networks / deep learning | Requires labeled FTP test data; cold start problem for new users |
| Heart rate decoupling | Requires consistent HR data; less precise than power alone |

**Cloud Enhancement**: Optional ML refinement post-ride using gradient boosting to detect physiological changes requiring FTP adjustment, trained on population-level adaptation patterns.

---

## 2. Real-Time Fatigue Detection

**Decision**: Multi-signal analysis with aerobic decoupling as primary indicator, power variability index (PVI) as secondary, and HRV as tertiary confirmation.

**Rationale**:
- Aerobic decoupling (HR drift vs. constant power) is the most validated real-time fatigue marker in cycling literature (Friel, Coggan)
- Power variability indicates neuromuscular fatigue onset
- HRV-based fatigue requires recent baseline and HR monitor connectivity

**Algorithm Stack**:

```
Primary: Aerobic Decoupling Detection
  - Monitor HR drift during steady-state efforts
  - Threshold: HR increase > 10% while power remains constant ±5%
  - Window: 5-minute rolling analysis (update every 30s)

Secondary: Power Variability Index (PVI)
  - Calculate coefficient of variation of power over 5-min window
  - Alert when PVI increases 40%+ above athlete's baseline
  - Baseline: Personal 90th percentile from last 20 rides

Tertiary: HRV-Based (when HR monitor available)
  - Real-time HRV calculation from R-R intervals
  - Threshold: 20% decrease from pre-ride baseline
  - Lower accuracy than aerobic decoupling; use as confirmation only
```

**Alert Behavior**: Dismissible with 5-10 minute cooldown before re-triggering if conditions persist.

**Alternatives Considered**:

| Method | Limitation |
|--------|-----------|
| Core temperature modeling | Requires skin temp sensors; not available on trainer |
| Lactate estimation | Indirect; requires power zones to be accurate |
| Pure ML anomaly detection | Requires 20+ baseline rides per athlete; cold start issue |

---

## 3. Workout Recommendation System

**Decision**: Content-based filtering with collaborative signals (hybrid approach).

**Rationale**:
- Cold start problem: New users have no history → content-based solves this with metadata
- Content-based rules are interpretable → riders understand "why" recommendations matter
- Collaborative layer added post-launch when sufficient user dataset exists

**Recommendation Pipeline**:

```
Input: User goal, recent CTL, current ACWR, target date, available time

Content-Based Filtering:
  1. Filter workout library by goal alignment
  2. Filter by energy system match (e.g., VO2max = 5-8 min intervals)
  3. Rank by:
     - TSS appropriate to current load (ACWR adjustment)
     - Duration fit to available time
     - Recent frequency (avoid same-type duplicates)
  4. Return top 3 ranked workouts with reasoning

Collaborative Layer (Future - after 100+ user base):
  - Recommend workouts completed by users with similar:
    - Fitness level (CTL within ±10)
    - Goal type (same target event type)
    - Power profile (rider type classification)
  - Weighting: 70% content-based + 30% collaborative
```

**Built-In Workout Library**:
- Minimum curated set: ~80 workouts covering all zones and goal types
- Categories: Recovery, Base, Sweet Spot, Threshold, VO2max, Sprint
- Each workout tagged with: Energy system, goal alignment, TSS range, difficulty tier
- Stored in SQLite; updatable post-launch without code changes

**Alternatives Considered**:

| Approach | Why Rejected or Deferred |
|----------|-------------------------|
| Pure collaborative filtering | Fails for new users; requires 100+ dataset |
| Reinforcement learning | Complex reward function; long training horizon |
| Matrix factorization (SVD) | Requires historical completion matrix; cold start unsolved |

---

## 4. Performance Trend Forecasting

**Decision**: EWMA projection with regime detection for plateau and detraining alerts.

**Rationale**:
- CTL already uses EWMA in analytics module (42-day decay)
- Extending EWMA 4-12 weeks forward is statistically sound for time-series
- Regime detection triggers intervention recommendations
- Simpler and more interpretable than black-box neural networks

**Algorithm**:

```
Step 1: Establish current fitness trajectory
  - Extract last 30 days of CTL values
  - Calculate linear trend (slope) via least squares

Step 2: Project forward
  - If slope > 0 (improving): Projected_CTL(t) = CTL_now + (slope × t)
  - If slope ≈ 0 (plateau): Flag plateau, suggest periodization
  - If slope < 0 (detraining): Alert with projected fitness loss

Step 3: Event-based forecasting
  - If target event date set:
    - Calculate required CTL at event date
    - Compare vs. projected CTL
    - Suggest training adjustments

Step 4: Confidence calculation
  - ±10% accuracy for 4-week horizons (SC-004 requirement)
  - Accuracy degrades beyond 8 weeks (~15%)
```

**Plateau Detection**:
- Trigger: abs(slope) < 0.5 CTL/day AND consistent riding history
- Recommendations: Add intensity variation, target different energy systems, consider deload

**Alternatives Considered**:

| Model | Limitation |
|-------|-----------|
| ARIMA | Overfits short cycling data; hard to incorporate external goals |
| LSTM/RNN | Requires 50+ weeks baseline; prone to overfitting |
| Kalman filtering | More complex; marginal improvement over EWMA for CTL |

---

## 5. Cloud API Design

**Decision**: RESTful microservice with request queuing, prediction caching, and graceful offline degradation.

**Rationale**:
- Cloud offloads complex ML (gradient boosting, anomaly detection)
- RESTful is simple to implement and debug
- Request queuing handles offline periods
- Local caching prevents "no predictions available" when cloud unavailable

**Architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│ RustRide Desktop Client                                     │
├─────────────────────────────────────────────────────────────┤
│ • Post-Ride Trigger → analytics/triggers.rs                 │
│ • Cloud API Client (reqwest)                                │
│ • Offline Queue (crossbeam channel)                         │
│ • Prediction Cache (SQLite)                                 │
└─────────────────────────────────────────────────────────────┘
                              ↓ HTTPS
┌─────────────────────────────────────────────────────────────┐
│ RustRide Cloud Backend (Python + FastAPI)                   │
├─────────────────────────────────────────────────────────────┤
│ • Request Handler + Validation                              │
│ • Feature Engineering                                       │
│ • ML Models (scikit-learn / XGBoost)                        │
│ • Response Cache (Redis)                                    │
└─────────────────────────────────────────────────────────────┘
```

**API Endpoints**:

| Endpoint | Purpose | Response Time Target |
|----------|---------|---------------------|
| POST /api/v1/predictions/ftp | FTP refinement | <5 seconds |
| POST /api/v1/predictions/fatigue | Fatigue classification | <2 seconds |
| POST /api/v1/recommendations/workouts | Workout ranking | <3 seconds |
| POST /api/v1/forecasts/ctltrend | CTL projection | <3 seconds |

**Offline Behavior**:
- Queue requests when offline detected
- Retry with exponential backoff (30s, 60s, 2m, 5m)
- Max queue: 50 requests
- Display cached predictions with "last updated" timestamp

**Cache Expiry**:
- FTP predictions: 7 days
- Fatigue indicators: 24 hours
- Workout recommendations: 1 hour

**Infrastructure**:

| Component | Technology |
|-----------|-----------|
| API Framework | FastAPI (Python) |
| ML Framework | scikit-learn + XGBoost |
| Inference Cache | Redis |
| Model Storage | S3 with versioning |
| Deployment | Docker + Kubernetes |

**Alternatives Considered**:

| Design | Why Not Selected |
|--------|-----------------|
| GraphQL API | RESTful sufficient; GraphQL adds complexity |
| gRPC | HTTP/REST fine for <5s latency target |
| On-device ML (ONNX) | Model updates too frequent; requires ~500MB disk |
| Serverless (Lambda) | Cold starts violate <5s requirement |

---

## Implementation Priority

| Phase | Features | Timeline |
|-------|----------|----------|
| 1 | FTP prediction + Fatigue detection UI | Weeks 1-4 |
| 2 | CTL forecasting + Cloud API skeleton | Weeks 5-8 |
| 3 | Workout recommendations + ML models | Weeks 9-16 |
| 4 | Collaborative filtering, advanced anomaly detection | Post-MVP |

---

## References

- Coggan, A.R. & Allen, H. (2010) - *Training and Racing with a Power Meter* (FTP methodology)
- Morton, R.H. (1996) - Hyperbolic critical power model (CP/W' foundation)
- Friel, J. & Cheng, J. (2018) - Aerobic decoupling validation
- TrainingPeaks database patterns (rider classification, ACWR thresholds)
