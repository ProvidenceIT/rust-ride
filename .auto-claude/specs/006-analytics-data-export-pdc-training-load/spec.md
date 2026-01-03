# Analytics Data Export (PDC/Training Load)

## Overview

Enable export of training analytics data (Power Duration Curve, training load history, CP model) to JSON/CSV formats for external analysis, backup, or sharing with coaches.

## Rationale

The leaderboards/export.rs module demonstrates a complete export pattern with LeaderboardExporter that exports to JSON and CSV. The analytics module has rich data structures (PowerDurationCurve, DailyLoad, CpModel) stored via AnalyticsStore. Applying the leaderboard export pattern to analytics enables data portability.

---
*This spec was created from ideation and is pending detailed specification.*
