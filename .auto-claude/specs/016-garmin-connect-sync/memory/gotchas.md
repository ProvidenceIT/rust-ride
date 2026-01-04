# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2026-01-04 13:34]
Cargo commands are not allowed in sandbox mode - cannot run cargo check/test/build directly

_Context: Tried to run cargo check to verify Garmin upload queue changes, but command is blocked by sandbox_
