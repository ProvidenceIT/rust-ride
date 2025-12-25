# Research: Training Science & Analytics

**Feature**: 003-training-analytics
**Date**: 2025-12-25

## Overview

This document captures research findings for implementing advanced training analytics. All algorithms are based on established sports science literature and industry-standard practices used in platforms like TrainingPeaks, WKO5, and Golden Cheetah.

---

## 1. Power Duration Curve (PDC)

### Decision
Implement Mean Maximal Power (MMP) curve using an efficient single-pass algorithm with duration-bucketed storage.

### Rationale
- MMP is the industry standard for PDC visualization
- Single-pass algorithm allows incremental updates when new rides are added
- Duration bucketing (logarithmic) reduces storage while maintaining precision at critical durations

### Algorithm

```
For each ride:
  For each duration d in [1s, 2s, 3s, 5s, 10s, 20s, 30s, 1min, 2min, 3min, 5min, 10min, 20min, 30min, 60min, 90min, 120min, ...]:
    Compute max average power over any window of duration d
    Update global PDC if new value exceeds stored value
```

**Efficient Window Maximum Algorithm**:
- Use monotonic deque for O(n) computation of all windows
- For each duration, maintain deque of (index, power) pairs
- Achieves O(n × k) where k = number of duration buckets (~30)

### Storage Design
```sql
CREATE TABLE power_duration_curve (
    user_id TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    max_power INTEGER NOT NULL,
    achieved_at TEXT NOT NULL,
    ride_id TEXT REFERENCES rides(id),
    PRIMARY KEY (user_id, duration_seconds)
);
```

### Alternatives Considered
- **Store all samples, compute on demand**: Rejected - too slow for large ride history
- **Store only at fixed durations (1s, 5s, 1min, 5min, 20min, 60min)**: Rejected - loses granularity for curve fitting

---

## 2. Critical Power / W' Model

### Decision
Implement 2-parameter hyperbolic model using nonlinear regression with at least 3 data points from PDC.

### Rationale
- Morton (1996) hyperbolic model is well-validated for durations 2-15 minutes
- Requires only PDC data - no special tests needed
- Provides both CP (sustainable power) and W' (anaerobic capacity in kJ)

### Algorithm

**Power-Duration Relationship**:
```
t = W' / (P - CP)

Where:
  t = time to exhaustion (seconds)
  P = power output (watts)
  CP = critical power (watts)
  W' = anaerobic work capacity (joules)
```

**Fitting Procedure** (Least Squares):
1. Select 3+ data points from PDC between 2-20 minutes
2. Transform to linear form: Work = CP × time + W'
3. Use linear regression: Work(t) = CP × t + W'
4. Extract CP (slope) and W' (intercept)

**Implementation**:
```rust
// Linear regression on (duration, work) pairs
// work = power × duration
fn fit_cp_model(pdc_points: &[(u32, u16)]) -> (f64, f64) {
    let points: Vec<(f64, f64)> = pdc_points.iter()
        .filter(|(d, _)| *d >= 120 && *d <= 1200) // 2-20 min
        .map(|(d, p)| (*d as f64, *p as f64 * *d as f64))
        .collect();

    // Standard least squares regression
    let (slope, intercept) = linear_regression(&points);
    (slope, intercept) // (CP, W')
}
```

### Time-to-Exhaustion Prediction
```
TTE(P) = W' / (P - CP)  for P > CP
TTE(P) = infinity       for P <= CP
```

### Alternatives Considered
- **3-parameter model (adds P_max)**: Rejected - requires very short efforts (<15s), often missing from indoor training
- **W'bal real-time tracking**: Deferred to future iteration (dFRC tracking)

---

## 3. FTP Auto-Detection

### Decision
Use 95% of best 20-minute power OR power at 45-60 minute duration from PDC, whichever is more reliable based on data availability.

### Rationale
- Traditional FTP test (20-min all-out × 0.95) often overestimates by 5-10%
- Using actual sustained efforts from training provides more accurate estimates
- Coggan's original definition: power sustainable for ~1 hour

### Algorithm

**Method 1: Best 20-minute Power**
```
FTP_estimate_1 = PDC[20min] × 0.95
```

**Method 2: Extended Duration (preferred when available)**
```
If PDC has data at 45-60 min durations:
    FTP_estimate_2 = average(PDC[45min], PDC[60min])
Else:
    FTP_estimate_2 = PDC[20min] × 0.90  // More conservative
```

**Confidence Scoring**:
```
confidence = f(data_recency, effort_count, duration_coverage)

High: >= 3 efforts in last 6 weeks with 20+ min near-max efforts
Medium: >= 2 efforts in last 8 weeks with 10+ min near-max efforts
Low: Limited recent data, may be stale
```

### Change Detection
```
If |new_ftp - current_ftp| > 5%:
    Notify user
    Require confirmation before updating zones
```

### Alternatives Considered
- **ML-based estimation**: Rejected - adds complexity, requires training data
- **Heart rate decoupling**: Deferred - requires HR data integration

---

## 4. Training Load (ATL/CTL/ACWR)

### Decision
Implement exponentially-weighted moving averages for ATL (7-day) and CTL (42-day), with ACWR as ratio.

### Rationale
- TSB (Training Stress Balance) model by Coggan/Allen is industry standard
- EWMA provides smooth curves without sharp discontinuities
- ACWR simplifies to ATL/CTL ratio for immediate interpretation

### Algorithm

**Daily TSS Aggregation**:
```
daily_tss[date] = sum(ride.tss for ride in rides_on_date)
```

**Acute Training Load (ATL)** - 7-day exponential decay:
```
ATL_today = ATL_yesterday × (1 - 2/(7+1)) + TSS_today × (2/(7+1))
         = ATL_yesterday × 0.75 + TSS_today × 0.25
```

**Chronic Training Load (CTL)** - 42-day exponential decay:
```
CTL_today = CTL_yesterday × (1 - 2/(42+1)) + TSS_today × (2/(42+1))
         = CTL_yesterday × 0.953 + TSS_today × 0.047
```

**Training Stress Balance (TSB)**:
```
TSB = CTL - ATL
```

**Acute:Chronic Workload Ratio**:
```
ACWR = ATL / CTL

Interpretation:
  ACWR < 0.8:  Undertrained (possible detraining)
  ACWR 0.8-1.0: Optimal recovery zone
  ACWR 1.0-1.3: Optimal training zone
  ACWR 1.3-1.5: Caution zone
  ACWR > 1.5:  High injury risk
```

### Cold Start Handling
```
If no historical data:
    CTL_initial = 0
    ATL_initial = 0
    First 28 days: Show "Building baseline" message
    After 28 days: Show full metrics
```

### Alternatives Considered
- **Simple rolling sum**: Rejected - creates discontinuities when rides fall out of window
- **7-day/28-day ratio (Gabbett)**: Considered but EWMA provides smoother curves

---

## 5. VO2max Estimation

### Decision
Use Hawley & Noakes formula relating 5-minute max power to VO2max.

### Rationale
- Well-validated formula for cycling
- Requires only power data and body weight
- Provides comparable metric to lab testing (within 5-10%)

### Formula

**Hawley & Noakes (1992)**:
```
VO2max (ml/kg/min) = (10.8 × P_5min / weight_kg) + 7

Where:
  P_5min = 5-minute max power from PDC (watts)
  weight_kg = rider's body weight (kg)
```

**Alternative Formula (Sitko et al.)**:
```
VO2max = 0.01141 × P_5min + 0.435
(returns L/min, divide by weight for ml/kg/min)
```

### Population Percentiles
Store reference tables for age/gender percentiles based on published norms.

```rust
fn vo2max_percentile(vo2max: f32, age: u8, is_male: bool) -> u8 {
    // Reference: ACSM Guidelines for Exercise Testing
    let percentile_table = match (is_male, age_bracket(age)) {
        (true, 20..30) => [(25, 38), (50, 44), (75, 51), (90, 57), (95, 62)],
        // ... additional brackets
    };
    interpolate_percentile(percentile_table, vo2max)
}
```

### Alternatives Considered
- **Ramp test extrapolation**: Rejected - requires specific protocol
- **HR-based estimation**: Rejected - less accurate than power-based

---

## 6. Rider Type Classification

### Decision
Classify riders based on power profile shape using ratios of PDC values at key durations.

### Rationale
- Coggan/Allen power profile quadrant analysis is well-established
- Uses 5s, 1min, 5min, 20min (FTP) reference points
- Provides actionable insights for training focus

### Classification Algorithm

```
1. Normalize all PDC values to watts/kg
2. Calculate ratios vs population norms:
   - Neuromuscular: 5s power / reference
   - Anaerobic: 1min power / reference
   - VO2max: 5min power / reference
   - Threshold: 20min power / reference

3. Classify based on strongest ratio:
   - Sprinter: Neuromuscular > 1.1 × others
   - Pursuiter: Anaerobic > 1.1 × others
   - Climber: VO2max or Threshold > 1.1 × others
   - Time Trialist: Threshold > 1.1 × others, flat PDC shape
   - All-Rounder: All ratios within 10% of each other
```

### Reference Values (by category)

| Duration | Cat 1 | Cat 2 | Cat 3 | Cat 4 | Cat 5 |
|----------|-------|-------|-------|-------|-------|
| 5s       | 23 W/kg | 20 W/kg | 17 W/kg | 14 W/kg | 11 W/kg |
| 1min     | 11 W/kg | 9.5 W/kg | 8 W/kg | 6.5 W/kg | 5 W/kg |
| 5min     | 7 W/kg | 6 W/kg | 5 W/kg | 4 W/kg | 3.2 W/kg |
| 20min    | 5.5 W/kg | 4.6 W/kg | 3.9 W/kg | 3.2 W/kg | 2.5 W/kg |

### Alternatives Considered
- **ML clustering**: Rejected - adds complexity, reference tables well-established
- **Heart rate integration**: Deferred - power-only analysis sufficient for MVP

---

## 7. Sweet Spot Recommendations

### Decision
Generate workout recommendations at 88-93% FTP based on current training load and available time.

### Rationale
- Sweet Spot (88-93% FTP) provides optimal training stimulus per unit time
- Balances training stress with recovery requirements
- Popular with time-constrained athletes

### Algorithm

**Target Power Range**:
```
SS_low = FTP × 0.88
SS_high = FTP × 0.93
```

**Duration Recommendations** (based on CTL and available time):
```
Low CTL (<40):
  Recommend: 2×15min or 3×12min intervals
  Weekly frequency: 2-3 sessions

Medium CTL (40-70):
  Recommend: 2×20min or 3×15min intervals
  Weekly frequency: 2-3 sessions

High CTL (>70):
  Recommend: 2×30min or 3×20min intervals
  Weekly frequency: 2-4 sessions
```

**Training Load Adjustment**:
```
If ACWR > 1.3:
  Reduce duration by 20%
  Add: "Consider recovery ride instead"

If ACWR < 0.8:
  Increase duration by 10%
  Add: "Building fitness - stay consistent"
```

### Alternatives Considered
- **Full periodization engine**: Deferred to future iteration
- **AI-driven recommendations**: Rejected - deterministic rules sufficient for MVP

---

## 8. Implementation Dependencies

### Calculation Order
```
1. Store ride samples → triggers PDC update
2. PDC complete → enables CP/W' calculation
3. CP calculated → enables TTE predictions
4. PDC complete → enables FTP auto-detection
5. FTP confirmed → enables zone recalculation
6. Daily TSS → enables ATL/CTL calculation
7. 28+ days data → enables ACWR display
8. PDC + weight → enables VO2max
9. PDC normalized → enables rider classification
10. FTP + CTL → enables Sweet Spot recommendations
```

### Data Requirements

| Feature | Min Data Required |
|---------|-------------------|
| PDC | 3+ rides with varied efforts |
| CP/W' | 3+ max efforts at 2-20 min durations |
| FTP Auto | 2 weeks of riding, 20+ min efforts |
| ACWR | 28 days of ride data |
| VO2max | Single 5-min max effort + body weight |
| Rider Type | 3+ months of varied efforts |
| Sweet Spot | Valid FTP |

---

## References

1. Morton, R.H. (1996). A 3-parameter critical power model. Ergonomics.
2. Coggan, A.R. & Allen, H. (2010). Training and Racing with a Power Meter.
3. Hawley, J.A. & Noakes, T.D. (1992). Peak power output predicts VO2max.
4. Gabbett, T.J. (2016). The training-injury prevention paradox.
5. ACSM Guidelines for Exercise Testing and Prescription (11th ed.).
