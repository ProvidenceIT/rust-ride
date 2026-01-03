# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2026-01-03 10:42]
When mocking React Native's Vibration API for tests, must set it directly on the RN module in jest.setup.js (ReactNative.Vibration = {...}) rather than using jest.mock on the internal path, as the preset mock structure doesn't include Vibration by default.

_Context: Testing useHaptics hook and WorkoutControlBar component_
