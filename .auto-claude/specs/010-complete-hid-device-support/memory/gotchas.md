# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2026-01-03 06:21]
HidInputReader.device_handles must use std::sync::Mutex (not tokio::sync::Mutex) because device.rs uses std::sync::Mutex and the handle is shared between them. The blocking read loop uses .lock() without await, which is the std::sync pattern.

_Context: HID module type consistency between input.rs and device.rs_
