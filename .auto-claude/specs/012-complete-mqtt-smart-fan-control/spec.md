# Complete MQTT Smart Fan Control

Finish the MQTT client implementation for smart fan integration. Fans should automatically adjust speed based on power output, heart rate, or manual control.

## Rationale
Smart fan integration improves training comfort and is a differentiating feature for a comprehensive training setup. Most competitors don't offer this level of home automation integration.

## User Stories
- As a cyclist, I want my smart fan to speed up when I'm working hard so that I stay cool without manual adjustment
- As a user with home automation, I want MQTT integration so that RustRide works with my existing setup

## Acceptance Criteria
- [ ] MQTT connection to broker works reliably
- [ ] Fan speed adjusts based on configured trigger (power/HR)
- [ ] Manual fan control override available
- [ ] Reconnection on connection loss
- [ ] Clear setup documentation for common smart fan setups
