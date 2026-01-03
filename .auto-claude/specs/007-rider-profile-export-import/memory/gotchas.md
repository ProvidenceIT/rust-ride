# Gotchas & Pitfalls

Things to watch out for in this codebase.

## [2026-01-02 23:02]
Subtasks T002, T003, T004 may have been pre-completed as part of T001 which created ProfileData, FtpHistoryEntry, and AvatarExport structs all together. Check existing code before implementing.

_Context: Phase 1 Export Data Structures - T001 commit included multiple struct definitions that overlap with subsequent subtasks._
