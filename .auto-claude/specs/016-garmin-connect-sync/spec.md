# Garmin Connect Sync

Add Garmin Connect API integration for uploading completed rides. Support automatic background sync to ensure rides are always backed up to Garmin's ecosystem.

## Rationale
Garmin has a massive ecosystem of users who track all activities in Garmin Connect. Native sync makes RustRide seamlessly integrate into their existing training workflow.

## User Stories
- As a Garmin user, I want my indoor rides in Garmin Connect alongside my outdoor rides so that my training history is complete
- As a cyclist, I want automatic sync so that I don't have to manually export and upload files

## Acceptance Criteria
- [ ] User can connect Garmin Connect account
- [ ] Rides upload to Garmin Connect automatically
- [ ] FIT files upload correctly with all data preserved
- [ ] Connection status visible in settings
- [ ] Failed uploads queue for retry
