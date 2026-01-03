# FIT File Export

Add FIT file format export alongside existing TCX and CSV formats. FIT is the industry standard for Garmin devices and provides better compatibility with fitness platforms.

## Rationale
FIT format is the gold standard for fitness data interchange. Adding this export option ensures maximum compatibility with Garmin Connect, TrainingPeaks, and other platforms, reinforcing RustRide's data portability advantage over locked-in competitors.

## User Stories
- As a Garmin user, I want to export my rides as FIT files so that I can upload them to Garmin Connect
- As a coach, I want FIT exports so that I can analyze rides in professional analysis software

## Acceptance Criteria
- [ ] Export menu includes FIT format option
- [ ] FIT files contain all ride data (power, HR, cadence, GPS if available)
- [ ] Exported FIT files validate with Garmin's FIT SDK validator
- [ ] FIT files upload successfully to Garmin Connect
- [ ] Lap markers and workout structure preserved in FIT export
