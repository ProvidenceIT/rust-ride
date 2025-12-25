# Feature Specification: Training Science & Analytics

**Feature Branch**: `003-training-analytics`
**Created**: 2025-12-25
**Status**: Draft
**Input**: User description: "Advanced training metrics and periodization features based on sports science research"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View Power Duration Curve (Priority: P1)

As a cyclist, I want to see my Power Duration Curve (PDC) showing maximum power outputs across all durations (5 seconds to multiple hours) so I can identify my strengths and weaknesses as a rider and track my fitness progression over time.

**Why this priority**: The PDC is foundational to all other analytics features. It provides the data needed for CP/W' calculations, rider type classification, and serves as the primary visualization for understanding power capabilities. Without historical power data analysis, other advanced metrics cannot be calculated.

**Independent Test**: Can be fully tested by recording several rides with varying efforts, then viewing the generated PDC chart. Delivers immediate value by showing rider's power profile.

**Acceptance Scenarios**:

1. **Given** a user with at least 3 recorded rides, **When** they navigate to the Analytics dashboard, **Then** they see their PDC plotted as a curve showing max power from 5s to their longest effort
2. **Given** a displayed PDC, **When** the user hovers over any point on the curve, **Then** they see the exact power value and duration for that point
3. **Given** a PDC view, **When** the user selects different time ranges (last 30/60/90 days, all time), **Then** the curve updates to reflect only data from that period
4. **Given** completed ride data, **When** new max power values are achieved, **Then** the PDC automatically updates to reflect the new personal bests

---

### User Story 2 - Real-Time Metrics Dashboard (Priority: P1)

As a cyclist during a workout, I want to see Normalized Power (NP), Training Stress Score (TSS), and Intensity Factor (IF) in real-time so I can quantify my workout intensity and compare efforts accurately.

**Why this priority**: NP/TSS/IF are the industry-standard metrics for workout quantification. These enable accurate comparison between workouts regardless of variability in power output, and form the basis for training load tracking.

**Independent Test**: Can be fully tested during any ride by observing real-time NP calculation and seeing TSS/IF update as the ride progresses. Delivers immediate value by showing workout intensity.

**Acceptance Scenarios**:

1. **Given** a user is riding with a power meter connected, **When** they view the ride screen, **Then** they see current NP calculated using 30-second rolling average
2. **Given** a ride in progress with FTP configured, **When** viewing metrics, **Then** IF is displayed as NP/FTP ratio
3. **Given** a ride in progress, **When** viewing metrics, **Then** TSS is displayed and updates every few seconds
4. **Given** a completed ride, **When** viewing the ride summary, **Then** final NP, IF, and TSS values are displayed and saved

---

### User Story 3 - Critical Power & W' Model (Priority: P2)

As a cyclist, I want my Critical Power (CP) and W' (anaerobic work capacity) calculated from my ride data so I can understand my sustainable power threshold and anaerobic reserve for precise pacing.

**Why this priority**: CP/W' provides a more accurate model than FTP alone for pacing efforts. It enables time-to-exhaustion predictions and is essential for advanced workout planning. Depends on PDC data from P1.

**Independent Test**: Can be tested by having sufficient ride data (3+ max efforts of varying durations), then viewing calculated CP and W' values with validation against known performance.

**Acceptance Scenarios**:

1. **Given** a user with at least 3 max efforts of different durations (e.g., 3min, 12min, 20min), **When** viewing the Analytics dashboard, **Then** CP (in watts) and W' (in kJ) are calculated and displayed
2. **Given** calculated CP/W' values, **When** user enters a target power, **Then** predicted time to exhaustion is displayed
3. **Given** calculated CP/W' values, **When** user enters a target duration, **Then** predicted sustainable power is displayed
4. **Given** new ride data with max efforts, **When** the model is recalculated, **Then** CP/W' values update to reflect improved (or changed) fitness

---

### User Story 4 - Automatic FTP Detection (Priority: P2)

As a cyclist, I want my FTP automatically estimated from my regular workout data so I can have an accurate FTP without needing to perform dedicated, exhausting FTP tests.

**Why this priority**: Manual FTP tests are physically demanding and often lead to overestimation. Auto-detection provides continuous FTP tracking without testing burden. Requires ride history from P1 stories.

**Independent Test**: Can be tested after accumulating 2+ weeks of varied riding by comparing auto-detected FTP to known performance benchmarks or previous test results.

**Acceptance Scenarios**:

1. **Given** a user with at least 2 weeks of ride data including varied intensities, **When** viewing their profile, **Then** an estimated FTP value is displayed with confidence indicator
2. **Given** a newly detected FTP, **When** it differs significantly from current FTP, **Then** user is notified and asked to confirm the update
3. **Given** auto-detected FTP is accepted, **When** viewing training zones, **Then** zones are automatically recalculated based on new FTP
4. **Given** the auto-detection system, **When** insufficient data exists, **Then** user is informed that more varied ride data is needed

---

### User Story 5 - Training Load Tracking (ACWR) (Priority: P3)

As a cyclist, I want to see my Acute:Chronic Workload Ratio so I can monitor my training load progression and avoid injury from training spikes.

**Why this priority**: ACWR provides critical injury prevention guidance. Requires accumulated TSS data over 28+ days, making it dependent on earlier priority stories being used consistently.

**Independent Test**: Can be tested after 28+ days of tracked rides by viewing ACWR value and alerts, then comparing against training changes made.

**Acceptance Scenarios**:

1. **Given** a user with at least 28 days of ride data, **When** viewing the Training Load dashboard, **Then** ACWR is displayed (7-day / 28-day TSS ratio)
2. **Given** displayed ACWR, **When** the ratio is between 0.8-1.3, **Then** it is shown in green (optimal zone)
3. **Given** displayed ACWR, **When** the ratio exceeds 1.5, **Then** a warning alert is displayed about injury risk
4. **Given** displayed ACWR, **When** the ratio is below 0.8, **Then** a notice indicates the user may be detraining

---

### User Story 6 - VO2max Estimation (Priority: P3)

As a cyclist, I want my VO2max estimated from my power data so I can track my aerobic fitness and compare my fitness level to population norms.

**Why this priority**: VO2max is a widely understood fitness metric that provides context beyond cycling-specific metrics. Requires sustained max effort data.

**Independent Test**: Can be tested after recording a 5-minute max effort by viewing the estimated VO2max value and comparing against external testing or known benchmarks.

**Acceptance Scenarios**:

1. **Given** a user with a recorded 5-minute max power effort, **When** viewing their fitness metrics, **Then** estimated VO2max is displayed in ml/kg/min
2. **Given** displayed VO2max, **When** viewing the value, **Then** a comparison to age/gender population percentiles is shown
3. **Given** multiple VO2max calculations over time, **When** viewing fitness trends, **Then** a chart shows VO2max progression

---

### User Story 7 - Sweet Spot Training Recommendations (Priority: P4)

As a cyclist, I want optimized Sweet Spot workout recommendations so I can efficiently build fitness with time-effective training.

**Why this priority**: Sweet Spot training is popular for time-constrained athletes. Recommendations depend on having accurate FTP (from P2) and understanding training load (from P3).

**Independent Test**: Can be tested by requesting a Sweet Spot workout recommendation and executing it, then verifying the intensity targets match 88-93% FTP.

**Acceptance Scenarios**:

1. **Given** a user with configured FTP, **When** requesting Sweet Spot workout recommendations, **Then** workouts are suggested with intervals at 88-93% FTP
2. **Given** workout recommendations, **When** viewing details, **Then** duration and frequency recommendations based on current training load are shown
3. **Given** a Sweet Spot workout in progress, **When** the user is in an interval, **Then** real-time feedback shows whether power is in the Sweet Spot zone

---

### User Story 8 - Rider Type Classification (Priority: P4)

As a cyclist, I want to see my rider type classification (sprinter, time trialist, climber, all-rounder) so I can understand my natural strengths and tailor my training accordingly.

**Why this priority**: Rider classification helps athletes understand their physiology and make informed training decisions. Derived from PDC analysis.

**Independent Test**: Can be tested by viewing classification after building sufficient PDC data, comparing the result against self-perceived strengths.

**Acceptance Scenarios**:

1. **Given** a user with sufficient PDC data (at least 3 months of riding), **When** viewing their profile, **Then** their rider type classification is displayed
2. **Given** displayed rider type, **When** viewing details, **Then** an explanation of what the classification means and how it was determined is shown
3. **Given** a rider classification, **When** viewing training recommendations, **Then** suggestions acknowledge the rider's strengths and weaknesses

---

### Edge Cases

- What happens when a user has no power meter data? System gracefully degrades to heart rate-based estimates where possible, clearly indicating reduced accuracy
- What happens when power data has gaps or sensor dropouts? System uses interpolation for short gaps (<10 seconds) and excludes longer gaps from calculations
- How does the system handle unrealistic power spikes (>2000W for non-sprint efforts)? Spikes are filtered as noise using configurable thresholds
- What happens when a user has insufficient data for a metric? Clear messaging indicates minimum data requirements with progress toward meeting them
- How are metrics handled for users who primarily do group rides vs structured training? Both riding styles contribute to PDC and load tracking; recommendations adapt to riding patterns

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST calculate Normalized Power using 30-second rolling average raised to 4th power, averaged, then 4th root applied
- **FR-002**: System MUST calculate Training Stress Score as (duration in hours × IF²) × 100
- **FR-003**: System MUST calculate Intensity Factor as NP divided by FTP
- **FR-004**: System MUST generate Power Duration Curve from all recorded max power values at each duration from 1 second to maximum ride duration
- **FR-005**: System MUST calculate Critical Power using hyperbolic model fitting from multiple max efforts (minimum 3 efforts of different durations)
- **FR-006**: System MUST calculate W' (W-prime) as the y-intercept of the power-duration relationship in kJ
- **FR-007**: System MUST provide time-to-exhaustion predictions given a target power above CP
- **FR-008**: System MUST estimate FTP from workout data without requiring dedicated FTP tests
- **FR-009**: System MUST notify users when auto-detected FTP differs significantly (>5%) from current value
- **FR-010**: System MUST calculate Acute Training Load as sum of TSS for previous 7 days
- **FR-011**: System MUST calculate Chronic Training Load as sum of TSS for previous 28 days divided by 4 (weekly average)
- **FR-012**: System MUST calculate ACWR as Acute Training Load divided by Chronic Training Load
- **FR-013**: System MUST display visual warnings when ACWR exceeds 1.5 (high injury risk)
- **FR-014**: System MUST estimate VO2max from 5-minute max power using body weight and standard conversion formulas
- **FR-015**: System MUST classify rider type based on PDC shape analysis (power ratios at different durations)
- **FR-016**: System MUST filter power values exceeding configurable spike threshold (default 2000W) as noise
- **FR-017**: System MUST persist all calculated metrics with ride data for historical analysis
- **FR-018**: System MUST allow users to select date ranges for PDC and analytics views
- **FR-019**: System MUST recalculate derived metrics (zones, recommendations) when FTP changes
- **FR-020**: System MUST display appropriate messaging when insufficient data exists for a metric

### Key Entities

- **PowerSample**: Individual power measurement with timestamp, used for all calculations
- **RideMetrics**: Calculated NP, TSS, IF, and other summary metrics for a single ride
- **PowerDurationCurve**: Collection of max power values at each duration, updated incrementally with new rides
- **CriticalPowerModel**: CP and W' values with calculation date and confidence indicator
- **TrainingLoad**: Daily, weekly, and rolling training stress values (ATL, CTL, ACWR)
- **FTPEstimate**: Auto-detected FTP value with timestamp, confidence score, and user acceptance status
- **RiderProfile**: Classification type, VO2max estimate, and user-configured values (weight, age, FTP)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view their complete Power Duration Curve within 3 seconds of navigating to the analytics screen
- **SC-002**: Real-time NP, TSS, and IF update at least every 5 seconds during a ride
- **SC-003**: CP/W' model predictions match actual performance within 5% error for 90% of users
- **SC-004**: Auto-detected FTP values are within 5% of dedicated test results for users who validate
- **SC-005**: 95% of users can understand their ACWR status (green/amber/red) without consulting documentation
- **SC-006**: All analytics calculations complete without perceptible delay on devices meeting minimum specifications
- **SC-007**: Users report increased confidence in pacing decisions after using CP/W' predictions (measured via in-app feedback)
- **SC-008**: Rider type classification aligns with user self-assessment in 80% of cases

## Assumptions

- Users have power meters or smart trainers capable of providing power data (heart rate-only users receive limited functionality)
- FTP auto-detection requires at least 2 weeks of varied riding data including some harder efforts
- CP/W' calculations require at least 3 distinct max efforts of different durations (ideally 3-5min, 10-15min, and 20+ min)
- Users will input accurate body weight for VO2max calculations
- Standard sports science formulas and algorithms are acceptable (Coggan NP/TSS, Morton CP model)
- PDC and training load features are most valuable for users who ride consistently (3+ times per week)

## Out of Scope

The following features from the input are explicitly out of scope for this specification and will be addressed in future iterations:

- HRV Training Readiness (requires external device integration)
- Periodization Builder (requires significant UI/UX design for multi-week planning)
- Aerobic Threshold (LT1) / Two-Threshold Model (requires lactate testing or advanced HR analysis)
- dFRC Real-Time Tracking (complex real-time W' depletion modeling)
- Race Simulation Builder (requires course data integration)
- Stamina/Fatigue Index (requires advanced fatigue modeling research)
- Sleep Integration (requires third-party wearable API integration)

These features are documented for future consideration but are not part of the current implementation scope.
