# Complete Strava OAuth Sync

Finish the Strava OAuth integration to automatically upload completed rides to Strava. This includes proper token refresh handling, background sync, and upload status feedback.

## Rationale
Users want to share rides with their existing Strava community and maintain their activity history. This directly addresses the data lock-in pain point from competitors like Zwift and TrainerRoad where export options are limited.

## User Stories
- As a cyclist, I want my rides to automatically sync to Strava so that I can share my activities with my community
- As a user, I want to see upload status so that I know my ride was successfully synced

## Acceptance Criteria
- [ ] User can connect Strava account via OAuth flow
- [ ] Rides automatically upload to Strava after completion
- [ ] OAuth tokens refresh automatically when expired
- [ ] Upload status is shown in ride history
- [ ] Failed uploads can be retried manually
