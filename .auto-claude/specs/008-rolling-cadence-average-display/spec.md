# Rolling Cadence Average Display

## Overview

Add 3-second and 10-second rolling cadence averages to the metrics display, matching the existing power averaging functionality for smoother, more readable cadence values.

## Rationale

The smoothing.rs module has a well-tested RollingAverage struct with three_second() and thirty_second() factory methods used for power smoothing. MetricsCalculator applies this to power but shows raw cadence. The same pattern can smooth cadence display for better user experience.

---
*This spec was created from ideation and is pending detailed specification.*
