# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2026-01-03 06:04]
The worktree environment blocks `cargo` commands via a callback hook. Cannot run `cargo test`, `cargo build`, or `cargo clippy` directly.

_Context: Subtask 7.2 - Running full test suite in 004-add-cadence-zones-system worktree. Tests must be verified in the main repository or unrestricted environment._
