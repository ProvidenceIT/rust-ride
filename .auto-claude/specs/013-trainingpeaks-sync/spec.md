# TrainingPeaks Sync

Add TrainingPeaks OAuth integration for uploading completed rides and downloading structured workout plans. Support both manual and automatic sync options.

## Rationale
TrainingPeaks is widely used by coached athletes and provides advanced training analytics. Supporting TrainingPeaks sync makes RustRide viable for serious athletes working with coaches, competing directly with TrainerRoad's value proposition.

## User Stories
- As a coached athlete, I want my rides to sync to TrainingPeaks so that my coach can see my training
- As a TrainingPeaks user, I want to download my workout plans so that I can execute them in RustRide

## Acceptance Criteria
- [ ] User can connect TrainingPeaks account via OAuth
- [ ] Completed rides upload to TrainingPeaks
- [ ] Workout plans can be downloaded from TrainingPeaks
- [ ] Downloaded workouts appear in workout library
- [ ] Sync status shown in UI
