# Workout Export to ZWO/MRC Format

## Overview

Add workout export functionality to complement existing ZWO and MRC parsers. Allow users to export built-in or custom workouts to Zwift (.zwo) or TrainerRoad (.mrc) formats for use in other applications.

## Rationale

The workouts module has well-established parsers (parse_zwo, parse_mrc) that convert file content to Workout structs. The recording module shows a clear export pattern with export_csv, export_tcx, export_fit functions. Combining these patterns enables reverse conversion - Workout struct to file content.

---
*This spec was created from ideation and is pending detailed specification.*
