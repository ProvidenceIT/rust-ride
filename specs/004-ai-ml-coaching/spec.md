# Feature Specification: AI & Machine Learning Coaching

**Feature Branch**: `004-ai-ml-coaching`
**Created**: 2025-12-25
**Status**: Draft
**Input**: User description: "AI & Machine Learning features for personalization and intelligent coaching including FTP prediction, adaptive workout recommendations, fatigue detection, performance forecasting, and rider profiling"

## Clarifications

### Session 2025-12-25

- Q: What types of training goals can riders set? → A: Comprehensive goal system including general fitness goals (improve endurance, lose weight, get faster), event-focused goals with target dates (race, century ride, gran fondo, time trial), and energy system goals (improve VO2max, build threshold, develop sprint).
- Q: Where should ML computations run? → A: Cloud-based (models run on server, requires connectivity, easier updates).
- Q: What is the source of workouts for recommendations? → A: Both user imports and built-in library (combined pool for recommendations).
- Q: How often should ML models update predictions? → A: Post-ride (recalculate predictions after each completed ride is saved).
- Q: How should fatigue alerts behave when dismissed? → A: Dismissible with cooldown (can dismiss, alert returns after 5-10 minutes if still fatigued).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - AI FTP Prediction (Priority: P1)

As a cyclist, I want the system to automatically predict my FTP based on my workout history, so I don't need to perform formal FTP tests to keep my training zones accurate.

**Why this priority**: FTP is the foundation of all training zone calculations. Accurate, automatic FTP prediction enables all other training features to function correctly without requiring riders to perform exhausting formal tests.

**Independent Test**: Record 2-3 weeks of varied workouts with power data, then verify the system predicts FTP within 5% of a formal test result.

**Acceptance Scenarios**:

1. **Given** a rider with at least 5 rides containing power data across varied intensities, **When** the system analyzes their workout history, **Then** it generates an FTP prediction with a confidence score.
2. **Given** a new FTP prediction differs by more than 3% from the current FTP, **When** the prediction is generated, **Then** the rider receives a notification suggesting they review their FTP setting.
3. **Given** a rider has insufficient workout variety (e.g., only easy rides), **When** the system attempts prediction, **Then** it clearly indicates insufficient data and what workout types are needed.

---

### User Story 2 - Real-Time Fatigue Detection (Priority: P1)

As a cyclist, I want the system to detect when I'm fatiguing during a ride, so I can adjust my effort to prevent overtraining or poor workout quality.

**Why this priority**: Real-time fatigue detection directly impacts workout safety and effectiveness. Detecting fatigue prevents injuries and ensures training quality.

**Independent Test**: During a workout, simulate fatigue patterns (heart rate drift, power decline) and verify the system alerts the rider appropriately.

**Acceptance Scenarios**:

1. **Given** a rider is performing a steady-state effort, **When** their heart rate rises by more than 10% while power remains constant (aerobic decoupling), **Then** the system displays a fatigue warning.
2. **Given** a rider's power variability increases significantly above their normal pattern, **When** this anomaly is detected, **Then** the system suggests ending the hard effort or taking recovery.
3. **Given** a rider has heart rate monitor connected, **When** HRV-derived fatigue indicators exceed thresholds, **Then** the system recommends reducing intensity.
4. **Given** a rider dismisses a fatigue warning, **When** fatigue indicators remain elevated, **Then** the alert reappears after 5-10 minutes.

---

### User Story 3 - Adaptive Workout Recommendations (Priority: P2)

As a cyclist, I want the system to recommend workouts based on my fitness level, recent training load, and goals, so I can train optimally without hiring a coach.

**Why this priority**: Workout recommendations personalize the training experience and help riders progress efficiently. Depends on FTP prediction for accurate intensity prescription.

**Independent Test**: Set a training goal, complete several workouts, and verify the system recommends appropriate next workouts that consider fatigue and progression.

**Acceptance Scenarios**:

1. **Given** a rider has a training goal and recent workout history, **When** they request a workout recommendation, **Then** the system suggests 2-3 appropriate workouts ranked by suitability from the combined pool of user imports and built-in library.
2. **Given** a rider's acute training load is high (ACWR > 1.3), **When** a workout is recommended, **Then** the system prioritizes recovery or easy endurance workouts.
3. **Given** a rider hasn't trained a specific energy system recently (e.g., VO2max), **When** their goal requires that system, **Then** the recommendation includes appropriate interval workouts.
4. **Given** a rider completes a recommended workout, **When** they view recommendations afterward, **Then** the next recommendations account for the completed work.

---

### User Story 4 - Performance Trend Forecasting (Priority: P2)

As a cyclist, I want to see projections of my fitness trajectory, so I can plan my training around target events and understand if I'm on track.

**Why this priority**: Forecasting helps riders plan training blocks and set realistic goals. Valuable for event preparation but not essential for daily training.

**Independent Test**: With 4+ weeks of consistent training data, verify the system projects fitness trends for the next 4-12 weeks with reasonable accuracy.

**Acceptance Scenarios**:

1. **Given** a rider has at least 4 weeks of training history, **When** they view the performance forecast, **Then** they see projected CTL/fitness for the next 4-12 weeks.
2. **Given** a rider's fitness improvement has stalled for 2+ weeks, **When** the system analyzes their data, **Then** it indicates a plateau and suggests training adjustments.
3. **Given** a rider stops training for several days, **When** detraining would impact their fitness significantly, **Then** the system alerts them about projected fitness loss.
4. **Given** a rider has a target event date entered, **When** viewing forecasts, **Then** they see whether current training trajectory will achieve target fitness.

---

### User Story 5 - Workout Difficulty Estimation (Priority: P3)

As a cyclist, I want to see estimated difficulty for workouts before starting, so I can choose appropriate workouts for how I feel today.

**Why this priority**: Difficulty estimation improves workout selection but is supplementary to core training functionality.

**Independent Test**: View a workout's predicted difficulty, complete the workout, and verify the prediction matched perceived exertion.

**Acceptance Scenarios**:

1. **Given** a workout file with defined intervals and targets, **When** a rider views the workout, **Then** they see a difficulty score (e.g., 1-10 scale) personalized to their fitness.
2. **Given** a rider's current fatigue state from recent training, **When** viewing workout difficulty, **Then** the estimate adjusts for their fatigued state.
3. **Given** two riders with different FTPs view the same workout, **When** difficulty is calculated, **Then** each sees a personalized difficulty based on their individual capabilities.

---

### User Story 6 - Power Curve Profiling & Rider Classification (Priority: P3)

As a cyclist, I want to understand my rider type (sprinter, climber, time trialist, all-rounder), so I can focus training on my strengths or address weaknesses.

**Why this priority**: Rider profiling provides valuable insights but builds on existing PDC functionality. Lower priority as basic classification exists in analytics module.

**Independent Test**: Accumulate sufficient power data across durations, verify the system classifies rider type and provides actionable insights.

**Acceptance Scenarios**:

1. **Given** a rider has power data across multiple durations (5s to 60min), **When** the system analyzes their power curve, **Then** it classifies their rider type with explanation.
2. **Given** a rider's power profile shows a clear weakness relative to their strengths, **When** viewing their profile, **Then** the system highlights improvement opportunities.
3. **Given** a rider's profile changes over time (e.g., improved sprint), **When** viewing historical data, **Then** they can track how their rider type has evolved.

---

### User Story 7 - Cadence & Technique Analysis (Priority: P4)

As a cyclist, I want feedback on my pedaling technique and optimal cadence, so I can improve efficiency and reduce injury risk.

**Why this priority**: Technique analysis is a refinement feature. Most riders benefit more from structured training than technique optimization.

**Independent Test**: Record workouts with varying cadences, verify the system identifies optimal cadence zones and technique patterns.

**Acceptance Scenarios**:

1. **Given** a rider has cadence data from multiple workouts at various intensities, **When** the system analyzes their data, **Then** it identifies their most efficient cadence range.
2. **Given** a rider's cadence variability increases late in workouts, **When** this pattern is detected, **Then** the system notes potential technique degradation under fatigue.
3. **Given** a rider consistently uses suboptimal cadence for their power output, **When** viewing analysis, **Then** the system suggests cadence targets for improvement.

---

### User Story 8 - Training Load Adaptation Engine (Priority: P4)

As a cyclist, I want personalized training load recommendations that adapt to my individual recovery capacity, so I can maximize gains while avoiding overtraining.

**Why this priority**: Personalized load optimization is advanced functionality building on basic ACWR tracking. Requires significant historical data.

**Independent Test**: Train consistently for 6+ weeks, verify the system learns individual recovery patterns and adjusts load recommendations accordingly.

**Acceptance Scenarios**:

1. **Given** a rider has 6+ weeks of training history with performance outcomes, **When** the system analyzes their data, **Then** it identifies their optimal training load range.
2. **Given** a rider's performance is declining despite consistent training, **When** the system detects this pattern, **Then** it recommends reducing training load.
3. **Given** a rider recovers faster than average between hard sessions, **When** the system models their adaptation, **Then** it allows higher training frequency recommendations.

---

### Edge Cases

- What happens when a rider has power data but no heart rate? System uses power-only analysis with reduced fatigue detection accuracy.
- How does the system handle sensor dropouts during analysis? Short gaps (< 10s) are interpolated; longer gaps mark affected segments as incomplete.
- What happens when predictions conflict (e.g., FTP prediction differs from recent test)? System shows both values with explanation and lets rider choose.
- How does the system handle riders returning from injury/illness? Reset adaptation models with user confirmation; use conservative predictions initially.
- What happens when insufficient data exists for predictions? System clearly indicates minimum data requirements and tracks progress toward them.
- What happens when cloud service is unavailable? System displays cached predictions with "last updated" timestamp; new predictions queued until connectivity restored.

## Requirements *(mandatory)*

### Functional Requirements

**Training Goals**
- **FR-026**: System MUST support general fitness goals (improve endurance, lose weight, get faster).
- **FR-027**: System MUST support event-focused goals with target dates (race, century ride, gran fondo, time trial).
- **FR-028**: System MUST support energy system goals (improve VO2max, build threshold, develop sprint).
- **FR-029**: System MUST allow riders to have multiple active goals simultaneously.

**FTP Prediction**
- **FR-001**: System MUST generate FTP predictions from workout history when rider has 5+ varied-intensity rides.
- **FR-002**: System MUST provide confidence scores (high/medium/low) with each FTP prediction.
- **FR-003**: System MUST notify riders when predicted FTP differs significantly (>3%) from current setting.
- **FR-004**: System MUST explain what data contributed to the prediction.
- **FR-030**: System MUST recalculate FTP predictions after each completed ride is saved.

**Fatigue Detection**
- **FR-005**: System MUST detect aerobic decoupling (HR drift vs power) during steady efforts.
- **FR-006**: System MUST identify abnormal power variability indicating fatigue.
- **FR-007**: System MUST provide real-time fatigue warnings during rides when thresholds exceeded.
- **FR-008**: System MUST track fatigue indicators across workout history for pattern recognition.
- **FR-031**: System MUST allow riders to dismiss fatigue alerts with a 5-10 minute cooldown before re-alerting.

**Workout Recommendations**
- **FR-009**: System MUST recommend workouts based on training goals, recent load, and fitness level.
- **FR-010**: System MUST consider ACWR when recommending workout intensity.
- **FR-011**: System MUST balance energy system development in recommendations.
- **FR-012**: System MUST explain why each workout was recommended.
- **FR-032**: System MUST recommend workouts from combined pool of user-imported workouts and built-in workout library.
- **FR-033**: System MUST include a built-in library of curated workouts covering all training zones and goals.

**Performance Forecasting**
- **FR-013**: System MUST project CTL/fitness trends for 4-12 weeks based on training patterns.
- **FR-014**: System MUST detect fitness plateaus and suggest interventions.
- **FR-015**: System MUST alert riders to detraining when training frequency drops.
- **FR-016**: System MUST allow setting target event dates for goal-oriented forecasting.

**Difficulty Estimation**
- **FR-017**: System MUST calculate personalized difficulty scores for workouts based on rider's current fitness.
- **FR-018**: System MUST adjust difficulty estimates based on current fatigue state.

**Rider Profiling**
- **FR-019**: System MUST classify rider type based on power duration curve analysis.
- **FR-020**: System MUST identify relative strengths and weaknesses in power profile.
- **FR-021**: System MUST track rider type evolution over time.

**Technique Analysis**
- **FR-022**: System MUST identify optimal cadence ranges from workout data.
- **FR-023**: System MUST detect technique degradation patterns under fatigue.

**Adaptation Engine**
- **FR-024**: System MUST learn individual recovery patterns from training response data.
- **FR-025**: System MUST personalize training load recommendations based on learned patterns.

**Cloud Infrastructure**
- **FR-034**: System MUST run ML model inference on cloud servers.
- **FR-035**: System MUST sync ride data to cloud for ML processing after ride completion.
- **FR-036**: System MUST cache predictions locally for offline viewing.
- **FR-037**: System MUST queue prediction requests when offline and process when connectivity restored.

### Key Entities

- **TrainingGoal**: Goal type (general/event/energy-system), target metrics, target date (for events), priority, status.
- **FtpPrediction**: Predicted FTP value, confidence level, contributing workout references, prediction date, method used.
- **FatigueState**: Current fatigue indicators (aerobic decoupling score, power variability index, HRV metrics if available), timestamp, ride reference, alert dismissal state.
- **WorkoutRecommendation**: Recommended workout reference, source (built-in/imported), suitability score, reasoning text, target energy systems, expected TSS.
- **PerformanceProjection**: Projected CTL values by date, confidence intervals, plateau indicators, detraining alerts.
- **DifficultyEstimate**: Workout reference, personalized difficulty score, adjustments applied (fatigue, fitness level).
- **RiderProfile**: Classification type, power profile percentiles at key durations, identified strengths/weaknesses, evolution history.
- **CadenceAnalysis**: Optimal cadence range, efficiency metrics by cadence band, technique patterns.
- **AdaptationModel**: Personalized recovery rate, optimal load parameters, training response history.
- **BuiltInWorkout**: Curated workout definition, target zone, goal alignment, difficulty tier.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: FTP predictions are within 5% of formal test results for 85% of riders with sufficient data.
- **SC-002**: Fatigue detection achieves 85-91% accuracy in identifying workout-affecting fatigue states.
- **SC-003**: Workout recommendations result in 80% user acceptance rate (riders complete recommended workouts).
- **SC-004**: Performance forecasts predict CTL within 10% accuracy over 4-week horizons.
- **SC-005**: Difficulty estimates correlate with rider-reported perceived exertion (RPE) with r > 0.7.
- **SC-006**: Rider type classifications are stable (consistent over 2+ week windows with similar training).
- **SC-007**: Cadence recommendations improve efficiency metrics for riders who follow guidance.
- **SC-008**: Personalized load recommendations reduce overtraining incidents by 30% vs generic recommendations.
- **SC-009**: System provides actionable insights within 2 seconds of data availability.
- **SC-010**: All predictions and recommendations include clear explanations riders can understand.
- **SC-011**: Cloud ML inference completes within 5 seconds of ride data upload.

## Assumptions

- Riders have power meters providing consistent, accurate data.
- Heart rate data is available for fatigue detection (degraded functionality without HR).
- Riders train regularly enough to generate sufficient data for predictions (minimum 3 rides/week for accurate analysis).
- Training goals are explicitly set by riders when using recommendation features.
- Historical ride data from previous features (PDC, training load) is available for analysis.
- Internet connectivity is available for ML processing (graceful degradation with cached data when offline).
