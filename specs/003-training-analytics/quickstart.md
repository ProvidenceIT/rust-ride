# Quickstart: Training Science & Analytics

**Feature**: 003-training-analytics
**Date**: 2025-12-25

## Prerequisites

- Rust 1.75+ installed
- RustRide codebase cloned and building
- Familiarity with existing `src/metrics/` module

## Development Setup

### 1. Switch to Feature Branch

```bash
git checkout 003-training-analytics
```

### 2. Verify Build

```bash
cargo build
cargo test
```

### 3. Module Structure

The analytics feature adds a new submodule under `src/metrics/`:

```
src/metrics/
├── mod.rs                 # Update to export analytics
├── calculator.rs          # [EXISTS] NP/TSS/IF
├── smoothing.rs           # [EXISTS] Rolling averages
├── zones.rs               # [EXISTS] Power/HR zones
└── analytics/             # [NEW] This feature
    ├── mod.rs
    ├── pdc.rs
    ├── critical_power.rs
    ├── ftp_detection.rs
    ├── training_load.rs
    ├── vo2max.rs
    ├── rider_type.rs
    └── sweet_spot.rs
```

---

## Implementation Order

Follow this order to minimize dependencies:

### Phase 1: Core Calculations (No Storage)

1. **pdc.rs** - MmpCalculator and PowerDurationCurve
   - Pure calculation, no database
   - Test with sample power arrays

2. **critical_power.rs** - CpFitter and CpModel
   - Depends on PDC types
   - Linear regression implementation

3. **training_load.rs** - EWMA calculations
   - Independent of PDC
   - Test with mock TSS data

4. **vo2max.rs** - Simple formula
   - Depends only on PDC for 5-min power
   - Quick to implement

### Phase 2: Detection & Classification

5. **ftp_detection.rs** - Auto FTP
   - Depends on PDC
   - Threshold detection logic

6. **rider_type.rs** - Classification
   - Depends on PDC
   - Reference table lookups

7. **sweet_spot.rs** - Recommendations
   - Depends on FTP and training load
   - Simple calculations

### Phase 3: Storage Integration

8. **analytics_store.rs** - Database layer
   - Implement after calculations work
   - Refer to `storage-api.md` contract

9. **Schema migration** - Add new tables
   - Update `schema.rs`
   - Version 1 → 2

### Phase 4: UI Integration

10. **analytics_screen.rs** - Dashboard
11. **pdc_chart.rs** - Visualization
12. **training_load_widget.rs** - ACWR display

---

## Quick Test: MMP Calculation

Create this test to verify core algorithm:

```rust
// tests/unit/analytics/pdc_tests.rs

use rustride::metrics::analytics::pdc::MmpCalculator;

#[test]
fn test_mmp_constant_power() {
    let calculator = MmpCalculator::standard();

    // 10 minutes of constant 200W
    let samples: Vec<u16> = vec![200; 600];
    let mmp = calculator.calculate(&samples);

    // Should find 200W at all durations up to 600s
    for point in &mmp {
        if point.duration_secs <= 600 {
            assert_eq!(point.power_watts, 200);
        }
    }
}

#[test]
fn test_mmp_variable_power() {
    let calculator = MmpCalculator::standard();

    // 5 min easy, 1 min hard, 5 min easy
    let mut samples = vec![150u16; 300]; // 5 min @ 150W
    samples.extend(vec![400u16; 60]);     // 1 min @ 400W
    samples.extend(vec![150u16; 300]);    // 5 min @ 150W

    let mmp = calculator.calculate(&samples);

    // 1-min max should be 400W
    let one_min = mmp.iter().find(|p| p.duration_secs == 60).unwrap();
    assert_eq!(one_min.power_watts, 400);

    // 5-min max should include some of the 400W effort
    let five_min = mmp.iter().find(|p| p.duration_secs == 300).unwrap();
    assert!(five_min.power_watts > 150);
}
```

---

## Key Patterns

### Using Existing Components

```rust
// Reuse existing smoothing for NP calculation
use crate::metrics::smoothing::NormalizedPowerCalculator;

// Reuse zone calculations
use crate::metrics::zones::PowerZones;
```

### Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
}
```

### Testing with Fixtures

```rust
// Load test data from fixtures
fn load_test_ride() -> Vec<u16> {
    let data = include_str!("../../../tests/fixtures/analytics/sample_ride.json");
    serde_json::from_str(data).unwrap()
}
```

---

## Reference Formulas

### Normalized Power
```
NP = (mean of (30s_rolling_avg ^ 4)) ^ 0.25
```

### TSS
```
TSS = (duration_hours × IF²) × 100
IF = NP / FTP
```

### Critical Power (linear regression)
```
Work = CP × time + W'
Where: work = power × time (joules)
```

### EWMA
```
ATL_today = ATL_yesterday × 0.75 + TSS_today × 0.25
CTL_today = CTL_yesterday × 0.953 + TSS_today × 0.047
```

### VO2max (Hawley-Noakes)
```
VO2max = (10.8 × P_5min / weight_kg) + 7
```

---

## Debugging Tips

### Validate PDC Extraction
```rust
// Check MMP is monotonically decreasing
fn validate_pdc(pdc: &[PdcPoint]) -> bool {
    pdc.windows(2).all(|w| w[0].power_watts >= w[1].power_watts)
}
```

### Debug CP Fitting
```rust
// Log fitting points
tracing::debug!(
    "Fitting CP with {} points: {:?}",
    points.len(),
    points
);
```

### Trace Training Load
```rust
// Verify EWMA progression
for (date, load) in history.iter() {
    tracing::trace!(
        "Date: {} TSS: {:.1} ATL: {:.1} CTL: {:.1}",
        date, load.tss, load.atl, load.ctl
    );
}
```

---

## Resources

- **Spec**: `specs/003-training-analytics/spec.md`
- **Research**: `specs/003-training-analytics/research.md`
- **Data Model**: `specs/003-training-analytics/data-model.md`
- **API Contract**: `specs/003-training-analytics/contracts/analytics-api.md`
- **Storage Contract**: `specs/003-training-analytics/contracts/storage-api.md`

---

## Common Issues

### "Insufficient data for CP"
- Need 3+ max efforts between 2-20 minutes
- Check PDC has points in this range

### "Training load shows zero"
- Requires 28+ days of data for meaningful ACWR
- Check daily TSS aggregation is working

### "VO2max seems wrong"
- Verify body weight is set correctly
- Check 5-minute power is reasonable (not a spike)

### "PDC chart is slow"
- Reduce number of duration buckets for display
- Cache computed PDC, update incrementally
