# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2026-01-03 10:42]
When mocking React Native's Vibration API for tests, must set it directly on the RN module in jest.setup.js (ReactNative.Vibration = {...}) rather than using jest.mock on the internal path, as the preset mock structure doesn't include Vibration by default.

_Context: Testing useHaptics hook and WorkoutControlBar component_

## [2026-01-03 14:22]
After ConnectionService.authenticate() returns, the service automatically sends subscribe_metrics and get_session_status requests. In tests, these must be handled by simulating the corresponding responses (subscribed_metrics and session_status) before making additional requests, otherwise tests will timeout waiting for pending promises.

_Context: Integration testing of ConnectionService authentication flow_
