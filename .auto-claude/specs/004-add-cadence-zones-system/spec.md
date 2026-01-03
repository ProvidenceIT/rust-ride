# Add Cadence Zones System

## Overview

Implement CadenceZones struct following the existing PowerZones and HRZones patterns. Cadence zones would track optimal cadence ranges for different training objectives (recovery, endurance, tempo climbing, high-cadence drills). Include zone tracking and change events similar to ZoneTracker's power/HR zone change detection.

## Rationale

The zones.rs module already has well-established patterns for PowerZones (7-zone Coggan) and HRZones (5-zone Karvonen) with color coding, zone lookup, and zone change events via ZoneTracker. The MetricsCalculator already processes cadence data but doesn't have zone classification. Extending this pattern to cadence would complete the metrics trifecta.

---
*This spec was created from ideation and is pending detailed specification.*
