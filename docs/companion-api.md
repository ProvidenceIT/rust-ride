# RustRide Companion API Reference

WebSocket API documentation for the RustRide Mobile Companion App protocol.

## Overview

The RustRide Companion API uses WebSocket for real-time bidirectional communication between the mobile companion app and the desktop application. All messages are JSON-encoded.

### Connection Details

| Property | Value |
|----------|-------|
| Protocol | WebSocket (ws://) |
| Default Port | 9876 |
| Service Discovery | mDNS (`_rustride._tcp.local.`) |
| Message Format | JSON |
| Metrics Push Rate | 1 Hz |

### Connection URL Format

```
ws://<host>:<port>
```

Example: `ws://192.168.1.100:9876`

---

## Authentication Flow

### Step 1: Connect

Establish a WebSocket connection to the server.

### Step 2: Authenticate (if PIN required)

If the server requires PIN authentication, send an `auth` request:

```json
{
  "type": "auth",
  "pin": "123456"
}
```

### Step 3: Handle Response

**Success Response:**
```json
{
  "type": "auth_ok",
  "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Failure Response:**
```json
{
  "type": "auth_failed",
  "reason": "Invalid PIN"
}
```

### Step 4: Subscribe to Metrics

After authentication, subscribe to receive real-time metrics:

```json
{
  "type": "subscribe_metrics"
}
```

### Unauthenticated Commands

The following commands do not require authentication:
- `ping` - Keep-alive ping
- `auth` - Authentication request

All other commands require successful authentication.

---

## Message Types

### Request Messages

Requests sent from the mobile client to the desktop server.

#### `auth` - Authenticate with PIN

Authenticate the client using a 6-digit PIN.

**Request:**
```json
{
  "type": "auth",
  "pin": "123456"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Must be `"auth"` |
| `pin` | string | Yes | 6-digit numeric PIN |

**Response:** `auth_ok` or `auth_failed`

---

#### `get_session_status` - Get Current Session State

Query the current workout/ride session status.

**Request:**
```json
{
  "type": "get_session_status"
}
```

**Response:**
```json
{
  "type": "session_status",
  "active": true,
  "session": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "session_type": "workout",
    "workout_name": "Sweet Spot",
    "workout_path": "/Users/cyclist/workouts/sweet_spot.zwo",
    "is_paused": false,
    "elapsed_secs": 1800,
    "current_interval_index": 3,
    "total_intervals": 10,
    "current_interval_name": "Threshold",
    "target_power_watts": 250,
    "interval_remaining_secs": 120
  }
}
```

**Response when no active session:**
```json
{
  "type": "session_status",
  "active": false,
  "session": null
}
```

---

#### `subscribe_metrics` - Subscribe to Real-time Metrics

Start receiving real-time metrics updates at 1Hz.

**Request:**
```json
{
  "type": "subscribe_metrics"
}
```

**Response:**
```json
{
  "type": "subscribed_metrics"
}
```

After subscription, the client will receive `metrics` events.

---

#### `unsubscribe_metrics` - Unsubscribe from Metrics

Stop receiving real-time metrics updates.

**Request:**
```json
{
  "type": "unsubscribe_metrics"
}
```

**Response:**
```json
{
  "type": "unsubscribed_metrics"
}
```

---

#### `workout_pause` - Pause Workout

Pause the active workout session.

**Request:**
```json
{
  "type": "workout_pause"
}
```

**Success Response:**
```json
{
  "type": "command_ok",
  "command": "workout_pause"
}
```

**Error Response (already paused):**
```json
{
  "type": "command_failed",
  "command": "workout_pause",
  "error": "Workout is already paused"
}
```

**Error Response (no session):**
```json
{
  "type": "error",
  "code": "NO_SESSION",
  "message": "No active session"
}
```

---

#### `workout_resume` - Resume Workout

Resume a paused workout session.

**Request:**
```json
{
  "type": "workout_resume"
}
```

**Success Response:**
```json
{
  "type": "command_ok",
  "command": "workout_resume"
}
```

**Error Response (not paused):**
```json
{
  "type": "command_failed",
  "command": "workout_resume",
  "error": "Workout is not paused"
}
```

---

#### `workout_skip` - Skip to Next Interval

Skip to the next interval in a structured workout.

**Request:**
```json
{
  "type": "workout_skip"
}
```

**Success Response:**
```json
{
  "type": "command_ok",
  "command": "workout_skip"
}
```

**Error Response (last interval):**
```json
{
  "type": "command_failed",
  "command": "workout_skip",
  "error": "Already at last interval"
}
```

**Error Response (not a workout):**
```json
{
  "type": "command_failed",
  "command": "workout_skip",
  "error": "Active session is not a workout"
}
```

---

#### `workout_stop` - Stop Session

Stop the active workout or free ride session.

**Request:**
```json
{
  "type": "workout_stop"
}
```

**Success Response:**
```json
{
  "type": "command_ok",
  "command": "workout_stop"
}
```

---

#### `adjust_resistance` - Adjust Trainer Resistance

Adjust the trainer resistance level during a free ride.

**Request:**
```json
{
  "type": "adjust_resistance",
  "delta": 5
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Must be `"adjust_resistance"` |
| `delta` | integer | Yes | Change amount (-100 to +100) |

**Success Response:**
```json
{
  "type": "command_ok",
  "command": "adjust_resistance"
}
```

---

#### `get_ride_history` - Get Ride History

Retrieve a paginated list of past rides.

**Request:**
```json
{
  "type": "get_ride_history",
  "limit": 20,
  "offset": 0
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Must be `"get_ride_history"` |
| `limit` | integer | Yes | Max rides to return (1-100) |
| `offset` | integer | Yes | Number of rides to skip (for pagination) |

**Response:**
```json
{
  "type": "ride_history",
  "rides": [
    {
      "ride_id": "550e8400-e29b-41d4-a716-446655440000",
      "started_at": "2024-01-15T10:00:00Z",
      "duration_secs": 3600,
      "distance_km": 25.5,
      "avg_power_watts": 200,
      "is_workout": true,
      "workout_name": "Sweet Spot"
    }
  ],
  "total": 42
}
```

---

#### `get_ride_details` - Get Ride Details

Retrieve detailed statistics for a specific ride.

**Request:**
```json
{
  "type": "get_ride_details",
  "ride_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Must be `"get_ride_details"` |
| `ride_id` | string | Yes | UUID of the ride |

**Response:**
```json
{
  "type": "ride_details",
  "ride": {
    "ride_id": "550e8400-e29b-41d4-a716-446655440000",
    "started_at": "2024-01-15T10:00:00Z",
    "ended_at": "2024-01-15T11:00:00Z",
    "duration_secs": 3600,
    "distance_km": 25.5,
    "calories": 650,
    "avg_power_watts": 200,
    "max_power_watts": 450,
    "normalized_power_watts": 215,
    "avg_heart_rate_bpm": 145,
    "max_heart_rate_bpm": 175,
    "avg_cadence_rpm": 88,
    "tss": 85.0,
    "intensity_factor": 0.92,
    "is_workout": true,
    "workout_name": "Sweet Spot"
  }
}
```

**Error Response (ride not found):**
```json
{
  "type": "error",
  "code": "INVALID_PARAMS",
  "message": "Ride not found: 550e8400-e29b-41d4-a716-446655440000"
}
```

---

#### `ping` - Keep-Alive Ping

Send a ping to keep the connection alive. Does not require authentication.

**Request:**
```json
{
  "type": "ping"
}
```

**Response:**
```json
{
  "type": "pong"
}
```

---

### Response Messages

Responses sent from the desktop server to the mobile client.

#### `auth_ok` - Authentication Successful

```json
{
  "type": "auth_ok",
  "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"auth_ok"` |
| `session_id` | string | UUID assigned to this connection |

---

#### `auth_failed` - Authentication Failed

```json
{
  "type": "auth_failed",
  "reason": "Invalid PIN format"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"auth_failed"` |
| `reason` | string | Human-readable failure reason |

---

#### `session_status` - Session Status Response

```json
{
  "type": "session_status",
  "active": true,
  "session": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"session_status"` |
| `active` | boolean | Whether a session is active |
| `session` | object\|null | Session details (see schema below) |

**SessionStatusInfo Schema:**

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | UUID of the session |
| `session_type` | string | `"free_ride"` or `"workout"` |
| `workout_name` | string\|null | Name of the workout (if applicable) |
| `workout_path` | string\|null | File path of the workout (if applicable) |
| `is_paused` | boolean | Whether the session is paused |
| `elapsed_secs` | integer | Total elapsed time in seconds |
| `current_interval_index` | integer\|null | Current interval (0-indexed) |
| `total_intervals` | integer\|null | Total number of intervals |
| `current_interval_name` | string\|null | Name of current interval |
| `target_power_watts` | integer\|null | Target power in ERG mode |
| `interval_remaining_secs` | integer\|null | Seconds remaining in interval |

---

#### `subscribed_metrics` - Metrics Subscription Confirmed

```json
{
  "type": "subscribed_metrics"
}
```

---

#### `unsubscribed_metrics` - Metrics Unsubscription Confirmed

```json
{
  "type": "unsubscribed_metrics"
}
```

---

#### `command_ok` - Command Executed Successfully

```json
{
  "type": "command_ok",
  "command": "workout_pause"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"command_ok"` |
| `command` | string | The command that was executed |

---

#### `command_failed` - Command Execution Failed

```json
{
  "type": "command_failed",
  "command": "workout_skip",
  "error": "Already at last interval"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"command_failed"` |
| `command` | string | The command that failed |
| `error` | string | Human-readable error message |

---

#### `ride_history` - Ride History Response

```json
{
  "type": "ride_history",
  "rides": [ ... ],
  "total": 42
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"ride_history"` |
| `rides` | array | Array of RideSummary objects |
| `total` | integer | Total number of rides (for pagination) |

**RideSummary Schema:**

| Field | Type | Description |
|-------|------|-------------|
| `ride_id` | string | UUID of the ride |
| `started_at` | string | ISO 8601 timestamp |
| `duration_secs` | integer | Duration in seconds |
| `distance_km` | number | Distance in kilometers |
| `avg_power_watts` | integer\|null | Average power in watts |
| `is_workout` | boolean | Whether this was a structured workout |
| `workout_name` | string\|null | Workout name (if applicable) |

---

#### `ride_details` - Ride Details Response

```json
{
  "type": "ride_details",
  "ride": { ... }
}
```

**RideDetailInfo Schema:**

| Field | Type | Description |
|-------|------|-------------|
| `ride_id` | string | UUID of the ride |
| `started_at` | string | ISO 8601 timestamp |
| `ended_at` | string | ISO 8601 timestamp |
| `duration_secs` | integer | Duration in seconds |
| `distance_km` | number | Distance in kilometers |
| `calories` | integer | Calories burned |
| `avg_power_watts` | integer\|null | Average power in watts |
| `max_power_watts` | integer\|null | Maximum power in watts |
| `normalized_power_watts` | integer\|null | Normalized power (NP) |
| `avg_heart_rate_bpm` | integer\|null | Average heart rate |
| `max_heart_rate_bpm` | integer\|null | Maximum heart rate |
| `avg_cadence_rpm` | integer\|null | Average cadence |
| `tss` | number\|null | Training Stress Score |
| `intensity_factor` | number\|null | Intensity Factor (IF) |
| `is_workout` | boolean | Whether this was a structured workout |
| `workout_name` | string\|null | Workout name (if applicable) |

---

#### `pong` - Ping Response

```json
{
  "type": "pong"
}
```

---

#### `error` - Error Response

```json
{
  "type": "error",
  "code": "AUTH_REQUIRED",
  "message": "Authentication required"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"error"` |
| `code` | string | Error code (see Error Codes) |
| `message` | string | Human-readable error message |

---

### Event Messages

Events pushed from the desktop server to subscribed clients.

#### `metrics` - Real-time Metrics Update

Pushed at 1Hz to subscribed clients during an active session.

```json
{
  "type": "metrics",
  "power_watts": 200,
  "heart_rate_bpm": 140,
  "cadence_rpm": 90,
  "speed_kmh": 32.5,
  "distance_km": 15.2,
  "elapsed_secs": 3600,
  "calories": 450
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"metrics"` |
| `power_watts` | integer\|null | Current power in watts |
| `heart_rate_bpm` | integer\|null | Current heart rate in BPM |
| `cadence_rpm` | integer\|null | Current cadence in RPM |
| `speed_kmh` | number\|null | Current speed in km/h |
| `distance_km` | number | Total distance in km |
| `elapsed_secs` | integer | Elapsed time in seconds |
| `calories` | integer | Calories burned |

Note: Sensor values are `null` when no sensor is connected.

---

#### `session_state_changed` - Session State Changed

Pushed when the session state changes (start, pause, resume, stop).

```json
{
  "type": "session_state_changed",
  "state": "active",
  "session": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"session_state_changed"` |
| `state` | string | New state (see Session States) |
| `session` | object\|null | Session details (if applicable) |

**Session States:**

| State | Description |
|-------|-------------|
| `idle` | No active session |
| `starting` | Session is starting |
| `active` | Session is running |
| `paused` | Session is paused |
| `stopping` | Session is stopping |
| `completed` | Session has completed |

---

#### `interval_changed` - Workout Interval Changed

Pushed when the workout advances to a new interval.

```json
{
  "type": "interval_changed",
  "interval_index": 3,
  "total_intervals": 10,
  "interval_name": "VO2max",
  "target_power_watts": 350,
  "duration_secs": 180
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"interval_changed"` |
| `interval_index` | integer | Current interval (0-indexed) |
| `total_intervals` | integer | Total number of intervals |
| `interval_name` | string | Name of the interval |
| `target_power_watts` | integer | Target power in watts |
| `duration_secs` | integer | Interval duration in seconds |

---

#### `disconnecting` - Connection Terminating

Pushed before the server closes the connection.

```json
{
  "type": "disconnecting",
  "reason": "Server shutting down"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `"disconnecting"` |
| `reason` | string | Reason for disconnection |

---

## Error Codes

| Code | Description |
|------|-------------|
| `AUTH_REQUIRED` | Authentication is required for this command |
| `INVALID_PIN` | The provided PIN is incorrect |
| `NO_SESSION` | No active workout/ride session |
| `SESSION_ACTIVE` | Cannot perform action - session already active |
| `UNKNOWN_COMMAND` | The command type is not recognized |
| `INVALID_PARAMS` | Invalid parameters in the request |
| `RATE_LIMITED` | Too many requests - slow down |
| `INTERNAL_ERROR` | Server encountered an internal error |

---

## QR Code Format

The QR code for quick pairing contains JSON data:

```json
{
  "url": "ws://192.168.1.100:9876",
  "pin": "123456",
  "version": "1"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | WebSocket connection URL |
| `pin` | string | No | 6-digit PIN (if authentication required) |
| `version` | string | Yes | Protocol version for compatibility |

---

## mDNS Service Discovery

The companion server advertises itself via mDNS for automatic discovery.

| Property | Value |
|----------|-------|
| Service Type | `_rustride._tcp.local.` |
| TXT Records | `port`, `version`, `protocol` |

**TXT Record Example:**
```
port=9876
version=1.0.0
protocol=rustride-companion-v1
```

---

## Connection Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│                    Connection Lifecycle                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. DISCOVER                                                │
│     └─ Scan for _rustride._tcp.local. services              │
│        OR scan QR code                                      │
│        OR enter IP:port manually                            │
│                                                             │
│  2. CONNECT                                                 │
│     └─ Establish WebSocket connection                       │
│                                                             │
│  3. AUTHENTICATE (if required)                              │
│     └─ Send: { "type": "auth", "pin": "..." }               │
│     └─ Receive: { "type": "auth_ok", ... }                  │
│                                                             │
│  4. SUBSCRIBE                                               │
│     └─ Send: { "type": "subscribe_metrics" }                │
│     └─ Send: { "type": "get_session_status" }               │
│                                                             │
│  5. OPERATE                                                 │
│     └─ Receive: metrics events at 1Hz                       │
│     └─ Send: control commands (pause, skip, stop)           │
│     └─ Receive: session state change events                 │
│                                                             │
│  6. KEEP-ALIVE                                              │
│     └─ Send: { "type": "ping" } every 30s                   │
│     └─ Receive: { "type": "pong" }                          │
│                                                             │
│  7. DISCONNECT                                              │
│     └─ Receive: { "type": "disconnecting", ... }            │
│     └─ Close WebSocket connection                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Best Practices

### Connection Management

1. **Reconnection** - Implement exponential backoff for reconnection attempts:
   - Initial delay: 1 second
   - Maximum delay: 30 seconds
   - Backoff multiplier: 2x

2. **Keep-Alive** - Send `ping` every 30 seconds to detect connection issues

3. **Timeout** - Set a 5-second timeout for request-response pairs

### Error Handling

1. **Authentication Errors** - Prompt user to re-enter PIN
2. **Command Failures** - Display error message to user
3. **Connection Loss** - Show reconnecting indicator and attempt reconnection
4. **Rate Limiting** - Implement request throttling

### State Management

1. **Optimistic Updates** - Update UI immediately for pause/resume
2. **State Sync** - Query `get_session_status` after reconnection
3. **Event Ordering** - Handle out-of-order event delivery gracefully

---

## Example: Complete Session Flow

```javascript
// 1. Connect
const ws = new WebSocket('ws://192.168.1.100:9876');

ws.onopen = () => {
  // 2. Authenticate
  ws.send(JSON.stringify({ type: 'auth', pin: '123456' }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  switch (message.type) {
    case 'auth_ok':
      // 3. Subscribe to metrics
      ws.send(JSON.stringify({ type: 'subscribe_metrics' }));
      ws.send(JSON.stringify({ type: 'get_session_status' }));
      break;

    case 'session_status':
      console.log('Session active:', message.active);
      if (message.session) {
        console.log('Workout:', message.session.workout_name);
      }
      break;

    case 'metrics':
      console.log('Power:', message.power_watts, 'W');
      console.log('HR:', message.heart_rate_bpm, 'bpm');
      break;

    case 'interval_changed':
      console.log('New interval:', message.interval_name);
      console.log('Target:', message.target_power_watts, 'W');
      break;

    case 'error':
      console.error('Error:', message.code, message.message);
      break;
  }
};

// 4. Control workout
function pauseWorkout() {
  ws.send(JSON.stringify({ type: 'workout_pause' }));
}

function skipInterval() {
  ws.send(JSON.stringify({ type: 'workout_skip' }));
}

// 5. Keep-alive
setInterval(() => {
  ws.send(JSON.stringify({ type: 'ping' }));
}, 30000);
```

---

## Version History

| Version | Changes |
|---------|---------|
| 1 | Initial protocol version |
