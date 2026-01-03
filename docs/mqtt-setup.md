# MQTT Smart Fan Setup Guide

This document describes how to set up and configure MQTT-based smart fan control with RustRide. Smart fan integration automatically adjusts your fan speed based on your training intensity, keeping you cool during hard efforts.

## Overview

RustRide can control smart fans via MQTT (Message Queuing Telemetry Transport), a lightweight messaging protocol commonly used in home automation. When configured, your fan speed will automatically adjust based on:

- **Power zones** - Fan speeds up as you hit harder efforts
- **Heart rate zones** - Fan speed follows your cardiovascular effort
- **Manual control** - Override automatic control during a ride

### Key Features

- Automatic fan speed adjustment based on training zones
- Configurable speed curves (customize speed for each zone)
- Multiple fan profile support
- Auto-reconnection on connection loss
- Secure authentication with password storage in OS keyring
- TLS/SSL support for encrypted connections
- Manual override during rides

---

## Prerequisites

Before setting up MQTT fan control in RustRide, you need:

1. **An MQTT broker** - Software that routes MQTT messages (covered in this guide)
2. **A smart fan or smart plug** - Any device controllable via MQTT, such as:
   - Tasmota-flashed smart plugs with PWM fans
   - Home Assistant-controlled fans
   - Smart fans with native MQTT support
   - DIY ESP8266/ESP32 fan controllers

---

## MQTT Broker Setup

### Option 1: Mosquitto (Standalone Broker)

Mosquitto is a lightweight, open-source MQTT broker that's perfect for home use.

#### Windows Installation

1. **Download Mosquitto** from [mosquitto.org/download](https://mosquitto.org/download/)
2. Run the installer
3. Open Services (`services.msc`) and ensure "Mosquitto Broker" is running
4. Default port: **1883** (unencrypted) or **8883** (TLS)

#### Linux Installation

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install mosquitto mosquitto-clients

# Start and enable the service
sudo systemctl start mosquitto
sudo systemctl enable mosquitto
```

#### macOS Installation

```bash
# Using Homebrew
brew install mosquitto

# Start the service
brew services start mosquitto
```

#### Mosquitto Configuration

Edit the Mosquitto configuration file:

- **Windows:** `C:\Program Files\mosquitto\mosquitto.conf`
- **Linux:** `/etc/mosquitto/mosquitto.conf`
- **macOS:** `/opt/homebrew/etc/mosquitto/mosquitto.conf`

**Basic configuration (no authentication):**

```conf
# Listen on all interfaces
listener 1883

# Allow anonymous connections (for testing only)
allow_anonymous true
```

**Secure configuration (with authentication):**

```conf
# Listen on all interfaces
listener 1883

# Disable anonymous access
allow_anonymous false

# Password file location
password_file /etc/mosquitto/passwd
```

Create a password file:

```bash
# Create a new password file with a user
sudo mosquitto_passwd -c /etc/mosquitto/passwd rustride

# Add additional users
sudo mosquitto_passwd /etc/mosquitto/passwd another_user
```

Restart Mosquitto after configuration changes:

```bash
# Linux
sudo systemctl restart mosquitto

# macOS
brew services restart mosquitto

# Windows - restart the service from services.msc
```

#### Testing Mosquitto

Verify the broker is working:

```bash
# In one terminal, subscribe to a test topic
mosquitto_sub -h localhost -t "test/topic"

# In another terminal, publish a message
mosquitto_pub -h localhost -t "test/topic" -m "Hello MQTT"

# With authentication:
mosquitto_sub -h localhost -t "test/topic" -u rustride -P yourpassword
mosquitto_pub -h localhost -t "test/topic" -m "Hello MQTT" -u rustride -P yourpassword
```

---

### Option 2: Home Assistant MQTT

If you already use Home Assistant for home automation, its built-in MQTT broker (Mosquitto add-on) is an excellent choice.

#### Installing the Mosquitto Add-on

1. Open Home Assistant
2. Go to **Settings** > **Add-ons** > **Add-on Store**
3. Search for "Mosquitto broker"
4. Click **Install**
5. After installation, click **Start**
6. Enable **Start on boot** and **Watchdog**

#### Configuring Authentication

1. Go to **Settings** > **People** > **Users**
2. Click **Add User**
3. Create a user for RustRide (e.g., username: `rustride`)
4. This user will automatically have MQTT access

#### Finding Your Broker Address

Your Home Assistant MQTT broker address is typically:

- **Same machine:** `localhost` or `127.0.0.1`
- **From another device:** Your Home Assistant IP address (e.g., `192.168.1.100`)
- **Default port:** `1883`

#### MQTT Integration in Home Assistant

For Home Assistant to see fan status from RustRide (optional):

1. Go to **Settings** > **Devices & Services**
2. Click **Add Integration**
3. Search for "MQTT"
4. Configure with the local broker details

---

## RustRide Configuration

### Enabling MQTT

1. Open RustRide
2. Go to **Settings** > **Integrations** > **MQTT / Smart Fan**
3. Toggle **Enable MQTT** on
4. Configure the following settings:

| Setting | Description | Example |
|---------|-------------|---------|
| **Broker Host** | Hostname or IP of your MQTT broker | `192.168.1.100` |
| **Port** | MQTT broker port | `1883` (standard) or `8883` (TLS) |
| **Use TLS** | Enable secure connection | Off for local, On for cloud |
| **Username** | Broker authentication username | `rustride` |
| **Password** | Broker authentication password | *(stored securely)* |
| **Client ID** | Unique identifier for RustRide | `rustride-abc123` |

### Testing the Connection

1. After entering your broker details, click **Test Connection**
2. You should see a green checkmark if successful
3. If it fails, check the error message for troubleshooting hints

### Password Storage

RustRide stores your MQTT password securely in your operating system's keyring:

- **Windows:** Windows Credential Manager
- **macOS:** macOS Keychain
- **Linux:** Secret Service (via libsecret/GNOME Keyring)

Your password is never stored in plain text configuration files.

---

## Fan Profile Configuration

### Creating a Fan Profile

1. In the MQTT settings section, scroll to **Fan Profiles**
2. Click **Add Fan Profile**
3. Configure the profile:

| Setting | Description |
|---------|-------------|
| **Name** | Friendly name for this fan (e.g., "Training Room Fan") |
| **MQTT Topic** | The topic your fan listens on |
| **Add /set Suffix** | Append "/set" to topic for commands |
| **Payload Format** | How speed commands are formatted |
| **Use Power Zones** | True = power zones, False = HR zones |
| **Change Delay** | Seconds to wait before changing speed |

### MQTT Topic Examples

Different devices use different topic structures:

| Device Type | Example Topic |
|-------------|---------------|
| Tasmota | `cmnd/living_room_fan/Dimmer` |
| Home Assistant | `homeassistant/fan/training_fan/set` |
| Generic | `home/fan/bedroom` |
| Custom | `rustride/fans/main` |

### Payload Formats

Choose the format your fan device expects:

| Format | Example Output | Use Case |
|--------|---------------|----------|
| **Speed Only** | `75` | Simple PWM controllers |
| **JSON Speed** | `{"speed": 75}` | Home Assistant fans |
| **JSON Speed + On/Off** | `{"speed": 75, "on": true}` | Fans with explicit on/off |
| **Percentage** | `75%` | Some smart plugs |

### Zone Speed Mapping

Configure what speed the fan should run at for each training zone:

| Zone | Default Speed | Description |
|------|---------------|-------------|
| Zone 1 (Recovery) | 0% | Fan off during recovery |
| Zone 2 (Endurance) | 20% | Light breeze |
| Zone 3 (Tempo) | 40% | Moderate airflow |
| Zone 4 (Threshold) | 60% | Good airflow |
| Zone 5 (VO2max) | 80% | Strong airflow |
| Zone 6 (Anaerobic) | 90% | Very strong |
| Zone 7 (Neuromuscular) | 100% | Maximum |

### Testing Your Fan

1. Select your fan profile
2. Click **Test Fan**
3. The fan will cycle through speeds: 25% > 50% > 75% > 100% > 50% > 0%
4. Verify the fan responds correctly at each speed

---

## Using Fan Control During Rides

### Automatic Mode

By default, fan control operates in automatic mode:

1. Start a ride
2. The fan speed adjusts based on your current zone
3. Changes occur with the configured delay (prevents rapid switching)
4. When you stop the ride, the fan turns off

### Manual Override

During a ride, you can override automatic control:

1. Press **G** to open the fan control panel (or click the fan icon)
2. Use the slider to set a manual speed
3. Use preset buttons: **Off** / **Low** / **Med** / **High** / **Max**
4. Click **Switch to Auto** to resume automatic control

### Status Indicator

During a ride, the fan status is shown in the top bar:

- **Fan Connected** (green) - MQTT connected, fan responsive
- **Connecting...** (blue) - Establishing connection
- **Reconnecting...** (yellow) - Lost connection, attempting to reconnect
- **Connection Lost** (red) - Connection failed
- **Fan Disconnected** (gray) - MQTT not configured or disabled

---

## Troubleshooting

### Connection Failed

**Symptoms:** "Connection failed" or "Connection timed out" error

**Solutions:**

1. **Verify broker is running:**
   ```bash
   # Check if Mosquitto is running
   systemctl status mosquitto  # Linux
   brew services list          # macOS
   sc query mosquitto          # Windows (in admin CMD)
   ```

2. **Check network connectivity:**
   ```bash
   # Test if broker port is reachable
   telnet your-broker-ip 1883
   nc -zv your-broker-ip 1883
   ```

3. **Verify broker address and port:**
   - Ensure the IP address is correct
   - Use `localhost` or `127.0.0.1` if broker is on same machine
   - Default port is 1883 (or 8883 for TLS)

4. **Check firewall settings:**
   - Windows: Allow Mosquitto through Windows Firewall
   - Linux: `sudo ufw allow 1883`
   - Router: Port forwarding if accessing remotely

### Authentication Failed

**Symptoms:** Connection rejected with authentication error

**Solutions:**

1. **Verify username and password:**
   - Re-enter the password in RustRide settings
   - Test with mosquitto_pub/sub command line tools

2. **Check Mosquitto password file:**
   ```bash
   # Re-create password for user
   sudo mosquitto_passwd /etc/mosquitto/passwd rustride
   sudo systemctl restart mosquitto
   ```

3. **Home Assistant users:**
   - Ensure the user exists under Settings > People > Users
   - The user must be a regular user, not just an admin

### Fan Not Responding

**Symptoms:** Connection successful but fan doesn't change speed

**Solutions:**

1. **Verify the MQTT topic:**
   - Use an MQTT client like MQTT Explorer to verify the topic
   - Subscribe to the topic and watch for messages from RustRide
   - Check your fan device documentation for the correct topic

2. **Check payload format:**
   - Different devices expect different formats
   - Use MQTT Explorer to see what your fan expects
   - Try different payload format options

3. **Test with command line:**
   ```bash
   # Test publishing directly
   mosquitto_pub -h localhost -t "home/fan/bedroom/set" -m '{"speed": 50}'
   ```

4. **Check fan device logs:**
   - Tasmota: Web console shows received commands
   - Home Assistant: Check the MQTT integration logs

### TLS Connection Issues

**Symptoms:** Connection fails when TLS is enabled

**Solutions:**

1. **Verify broker has TLS configured:**
   - Check broker configuration for certificate settings
   - Ensure port 8883 is used for TLS

2. **Certificate issues:**
   - RustRide uses system certificates
   - Self-signed certificates may not work without additional setup
   - Consider using Let's Encrypt for valid certificates

3. **Disable TLS for local connections:**
   - If broker is on your local network, TLS may be unnecessary
   - Toggle "Use TLS" off and use port 1883

### Reconnection Issues

**Symptoms:** Fan control stops working after temporary network issues

**Solutions:**

1. **Check reconnection settings:**
   - Verify reconnect interval is reasonable (5-30 seconds)
   - Check max reconnection attempts isn't set too low

2. **Network stability:**
   - Ensure stable WiFi connection
   - Consider using wired Ethernet for broker

3. **Check broker logs:**
   ```bash
   # View Mosquitto logs
   sudo journalctl -u mosquitto -f  # Linux
   tail -f /var/log/mosquitto/mosquitto.log
   ```

---

## Configuration File Reference

MQTT settings are saved in your RustRide configuration file:

**Location:**
- **Windows:** `%APPDATA%\RustRide\config.toml`
- **macOS:** `~/Library/Application Support/RustRide/config.toml`
- **Linux:** `~/.config/rustride/config.toml`

**Example MQTT section:**

```toml
[mqtt]
enabled = true
broker_host = "192.168.1.100"
broker_port = 1883
use_tls = false
username = "rustride"
client_id = "rustride-abc123"
reconnect_interval_secs = 5
keep_alive_secs = 60
connection_timeout_secs = 30
```

**Note:** The password is stored in your OS keyring, not in this file.

---

## Home Assistant Integration

This section provides detailed instructions for integrating RustRide with Home Assistant-controlled fans. Home Assistant is a popular open-source home automation platform that can control a wide variety of fan devices.

### Prerequisites

1. Home Assistant installed and running
2. Mosquitto MQTT broker add-on installed (see "Option 2: Home Assistant MQTT" above)
3. MQTT integration configured in Home Assistant
4. A fan device or smart plug added to Home Assistant

### Step 1: Set Up Your Fan in Home Assistant

Before connecting RustRide, ensure your fan is properly configured in Home Assistant.

#### Option A: Native Fan Entity

If your fan device is already in Home Assistant (e.g., Zigbee fan, WiFi smart fan):

1. Go to **Settings** > **Devices & Services**
2. Find your fan device
3. Note the entity ID (e.g., `fan.training_room_fan`)

#### Option B: Smart Plug as Fan Controller

If using a smart plug to control a regular fan:

1. Add your smart plug to Home Assistant (via Zigbee, WiFi, or Z-Wave)
2. Create a template fan entity in `configuration.yaml`:

```yaml
fan:
  - platform: template
    fans:
      training_fan:
        friendly_name: "Training Room Fan"
        unique_id: training_fan
        value_template: "{{ states('switch.training_plug') }}"
        turn_on:
          service: switch.turn_on
          target:
            entity_id: switch.training_plug
        turn_off:
          service: switch.turn_off
          target:
            entity_id: switch.training_plug
        # For on/off plugs, no speed control
        speed_count: 1
```

#### Option C: PWM-Controlled Fan (Advanced)

For fans with variable speed control via PWM:

```yaml
fan:
  - platform: template
    fans:
      pwm_training_fan:
        friendly_name: "PWM Training Fan"
        unique_id: pwm_training_fan
        value_template: "{{ states('switch.fan_relay') }}"
        percentage_template: "{{ states('input_number.fan_speed') | int }}"
        turn_on:
          service: switch.turn_on
          target:
            entity_id: switch.fan_relay
        turn_off:
          service: switch.turn_off
          target:
            entity_id: switch.fan_relay
        set_percentage:
          service: input_number.set_value
          target:
            entity_id: input_number.fan_speed
          data:
            value: "{{ percentage }}"
        speed_count: 100

input_number:
  fan_speed:
    name: Fan Speed
    min: 0
    max: 100
    step: 1
    mode: slider
```

### Step 2: Configure MQTT Discovery for Your Fan

Home Assistant can automatically expose fans via MQTT using MQTT discovery. Add this to your `configuration.yaml`:

```yaml
mqtt:
  fan:
    - name: "Training Fan MQTT"
      unique_id: training_fan_mqtt
      state_topic: "homeassistant/fan/training_fan/state"
      command_topic: "homeassistant/fan/training_fan/set"
      percentage_state_topic: "homeassistant/fan/training_fan/percentage_state"
      percentage_command_topic: "homeassistant/fan/training_fan/percentage/set"
      json_attributes_topic: "homeassistant/fan/training_fan/attributes"
      payload_on: "ON"
      payload_off: "OFF"
      speed_range_min: 0
      speed_range_max: 100
```

After adding this configuration, restart Home Assistant.

### Step 3: Create an Automation to Control Your Physical Fan

This automation listens to the MQTT topic that RustRide publishes to and controls your actual fan:

```yaml
automation:
  - id: rustride_fan_control
    alias: "RustRide Fan Control"
    description: "Control training room fan based on RustRide MQTT commands"
    trigger:
      - platform: mqtt
        topic: "homeassistant/fan/training_fan/percentage/set"
    action:
      - service: fan.set_percentage
        target:
          entity_id: fan.training_room_fan
        data:
          percentage: "{{ trigger.payload | int }}"

  - id: rustride_fan_on_off
    alias: "RustRide Fan On/Off"
    description: "Turn training fan on or off based on RustRide commands"
    trigger:
      - platform: mqtt
        topic: "homeassistant/fan/training_fan/set"
    action:
      - choose:
          - conditions:
              - condition: template
                value_template: "{{ trigger.payload | from_json | default({}) | selectattr('on') | list | length > 0 and (trigger.payload | from_json).on == true }}"
            sequence:
              - service: fan.turn_on
                target:
                  entity_id: fan.training_room_fan
          - conditions:
              - condition: template
                value_template: "{{ trigger.payload | from_json | default({}) | selectattr('on') | list | length > 0 and (trigger.payload | from_json).on == false }}"
            sequence:
              - service: fan.turn_off
                target:
                  entity_id: fan.training_room_fan
```

### Step 4: Configure RustRide for Home Assistant

In RustRide Settings > MQTT / Smart Fan:

| Setting | Value |
|---------|-------|
| **Broker Host** | Your Home Assistant IP (e.g., `192.168.1.100`) |
| **Port** | `1883` |
| **Username** | Your Home Assistant username |
| **Password** | Your Home Assistant password |
| **Use TLS** | Off (for local network) |

For the fan profile:

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `homeassistant/fan/training_fan/percentage` |
| **Add /set Suffix** | Yes |
| **Payload Format** | Speed Only |

### Step 5: Test the Integration

1. In RustRide, click **Test Connection** to verify MQTT connectivity
2. Click **Test Fan** to cycle through speeds
3. Verify your physical fan responds to each speed change

### Example Automations

Here are additional Home Assistant automations that enhance RustRide integration:

#### Automation 1: Turn Off Fan When RustRide Disconnects

Automatically turn off the fan if RustRide loses connection:

```yaml
automation:
  - id: fan_safety_shutoff
    alias: "Fan Safety Shutoff on RustRide Disconnect"
    description: "Turn off training fan if no updates received for 5 minutes"
    trigger:
      - platform: state
        entity_id: binary_sensor.rustride_connected
        to: "off"
        for:
          minutes: 5
    action:
      - service: fan.turn_off
        target:
          entity_id: fan.training_room_fan
      - service: notify.mobile_app
        data:
          message: "Training fan turned off - RustRide disconnected"
```

#### Automation 2: Announce Workout Zone Changes

Use text-to-speech to announce when you enter harder zones:

```yaml
automation:
  - id: announce_hard_zones
    alias: "Announce Hard Workout Zones"
    description: "TTS announcement when entering Zone 5+"
    trigger:
      - platform: mqtt
        topic: "homeassistant/fan/training_fan/percentage/set"
    condition:
      - condition: template
        value_template: "{{ trigger.payload | int >= 80 }}"
    action:
      - service: tts.speak
        target:
          entity_id: tts.google_en_com
        data:
          message: "Great effort! Entering high intensity zone."
          media_player_entity_id: media_player.training_room_speaker
```

#### Automation 3: Sync Multiple Fans

Control multiple fans together (e.g., main fan plus a desk fan):

```yaml
automation:
  - id: sync_training_fans
    alias: "Sync Training Room Fans"
    description: "Keep multiple fans in sync with RustRide commands"
    trigger:
      - platform: mqtt
        topic: "homeassistant/fan/training_fan/percentage/set"
    action:
      - service: fan.set_percentage
        target:
          entity_id:
            - fan.training_room_main
            - fan.training_room_desk
            - fan.garage_gym_fan
        data:
          percentage: "{{ trigger.payload | int }}"
```

#### Automation 4: Lighting Based on Effort

Change room lighting color based on training intensity:

```yaml
automation:
  - id: training_lights_by_zone
    alias: "Training Room Lights by Zone"
    description: "Change light color based on training intensity"
    trigger:
      - platform: mqtt
        topic: "homeassistant/fan/training_fan/percentage/set"
    action:
      - choose:
          - conditions:
              - condition: template
                value_template: "{{ trigger.payload | int <= 20 }}"
            sequence:
              - service: light.turn_on
                target:
                  entity_id: light.training_room
                data:
                  rgb_color: [0, 255, 0]  # Green - easy
                  brightness: 128
          - conditions:
              - condition: template
                value_template: "{{ trigger.payload | int <= 60 }}"
            sequence:
              - service: light.turn_on
                target:
                  entity_id: light.training_room
                data:
                  rgb_color: [255, 255, 0]  # Yellow - moderate
                  brightness: 192
          - conditions:
              - condition: template
                value_template: "{{ trigger.payload | int <= 80 }}"
            sequence:
              - service: light.turn_on
                target:
                  entity_id: light.training_room
                data:
                  rgb_color: [255, 165, 0]  # Orange - hard
                  brightness: 255
          - conditions:
              - condition: template
                value_template: "{{ trigger.payload | int > 80 }}"
            sequence:
              - service: light.turn_on
                target:
                  entity_id: light.training_room
                data:
                  rgb_color: [255, 0, 0]  # Red - max effort
                  brightness: 255
```

#### Automation 5: Log Workout Data

Record training session data for analysis:

```yaml
automation:
  - id: log_training_intensity
    alias: "Log Training Intensity"
    description: "Record fan speed changes to logbook for workout analysis"
    trigger:
      - platform: mqtt
        topic: "homeassistant/fan/training_fan/percentage/set"
    action:
      - service: logbook.log
        data:
          name: "Training Intensity"
          message: "Fan speed set to {{ trigger.payload }}%"
          entity_id: fan.training_room_fan
          domain: fan
```

### Creating a RustRide Status Sensor

Create a sensor in Home Assistant to track RustRide connection status:

```yaml
mqtt:
  sensor:
    - name: "RustRide Status"
      unique_id: rustride_status
      state_topic: "rustride/status"
      value_template: "{{ value_json.state | default('unknown') }}"
      json_attributes_topic: "rustride/status"

  binary_sensor:
    - name: "RustRide Connected"
      unique_id: rustride_connected
      state_topic: "rustride/status"
      value_template: "{{ value_json.state | default('disconnected') }}"
      payload_on: "connected"
      payload_off: "disconnected"
      device_class: connectivity
```

### Dashboard Card Example

Add a RustRide control card to your Home Assistant dashboard:

```yaml
type: vertical-stack
cards:
  - type: entities
    title: RustRide Training
    entities:
      - entity: binary_sensor.rustride_connected
        name: Connection Status
      - entity: fan.training_room_fan
        name: Training Fan
  - type: horizontal-stack
    cards:
      - type: button
        name: Fan Off
        icon: mdi:fan-off
        tap_action:
          action: call-service
          service: fan.turn_off
          target:
            entity_id: fan.training_room_fan
      - type: button
        name: 25%
        icon: mdi:fan-speed-1
        tap_action:
          action: call-service
          service: fan.set_percentage
          target:
            entity_id: fan.training_room_fan
          data:
            percentage: 25
      - type: button
        name: 50%
        icon: mdi:fan-speed-2
        tap_action:
          action: call-service
          service: fan.set_percentage
          target:
            entity_id: fan.training_room_fan
          data:
            percentage: 50
      - type: button
        name: Max
        icon: mdi:fan-speed-3
        tap_action:
          action: call-service
          service: fan.set_percentage
          target:
            entity_id: fan.training_room_fan
          data:
            percentage: 100
```

### Troubleshooting Home Assistant Integration

#### Fan Not Responding to RustRide Commands

1. **Verify MQTT messages are being received:**
   - Install the MQTT Explorer add-on or use MQTT.fx
   - Subscribe to `homeassistant/fan/#` to see all messages
   - Start a ride in RustRide and check for messages

2. **Check automation traces:**
   - Go to **Settings** > **Automations**
   - Click on your RustRide automation
   - Click **Traces** to see execution history

3. **Verify topic matches:**
   - RustRide topic must exactly match the trigger topic in your automation
   - Check for typos in topic names

#### Home Assistant User Authentication Issues

1. **Create a dedicated MQTT user:**
   - Go to **Settings** > **People** > **Users**
   - Create a user specifically for RustRide
   - Do not use an admin account

2. **Check Mosquitto add-on logs:**
   - Go to **Settings** > **Add-ons** > **Mosquitto broker**
   - Click on **Log** tab
   - Look for authentication failures

#### Payload Format Mismatches

If your fan expects JSON but receives plain numbers (or vice versa):

1. In RustRide, try different **Payload Format** options
2. Use MQTT Explorer to see what format RustRide is sending
3. Adjust your Home Assistant automation to parse the correct format:

```yaml
# For JSON payloads like {"speed": 75}
value_template: "{{ trigger.payload | from_json | selectattr('speed') | map(attribute='speed') | first | default(0) }}"

# For plain number payloads like 75
value_template: "{{ trigger.payload | int }}"
```

---

## Tasmota and Smart Plug Integration

This section provides detailed instructions for using Tasmota-flashed devices and other smart plugs to control fans with RustRide. This is one of the most cost-effective ways to add smart fan control to your training setup.

### What is Tasmota?

[Tasmota](https://tasmota.github.io/) is open-source firmware for ESP8266/ESP32-based devices. When flashed onto compatible smart plugs, switches, or dimmers, it enables full local control via MQTT without cloud dependencies.

**Benefits of Tasmota:**
- No cloud account required - fully local control
- Native MQTT support with configurable topics
- Works with most ESP8266/ESP32 smart devices
- Active community with regular updates
- Completely free and open source

### Supported Device Types

Tasmota-based fan control works with several device categories:

| Device Type | Fan Control | Speed Control | Best For |
|-------------|-------------|---------------|----------|
| **Smart Plug (on/off)** | Yes | No (on/off only) | Simple box fans |
| **Smart Dimmer Plug** | Yes | Yes (0-100%) | Dimmable fans, LED-controlled fans |
| **Smart Dimmer Switch** | Yes | Yes (0-100%) | Wall-mounted fans with dimmers |
| **PWM Controller Module** | Yes | Yes (0-100%) | DIY projects, 12V/24V fans |

### Compatible Devices

The following devices are known to work well with Tasmota for fan control:

**Smart Plugs (On/Off Only):**
- Sonoff S31 / S31 Lite
- Sonoff S26
- Gosund WP3 / WP6
- Teckin SP10 / SP20
- KMC Smart Plug

**Dimmer Plugs/Switches (Variable Speed):**
- Sonoff D1 Dimmer
- Martin Jerry Dimmer
- Treatlife Dimmer
- Zemismart Dimmer

**PWM Controllers:**
- Sonoff 4CH Pro (for multi-fan setups)
- Generic ESP8266 modules with MOSFETs

> **Note:** Always verify Tasmota compatibility before purchasing. Check the [Tasmota Device Templates](https://templates.blakadder.com/) database.

### Flashing Tasmota

#### Prerequisites

1. **Compatible device** from the list above (or check templates database)
2. **USB-to-serial adapter** (CP2102 or CH340 based) - for devices requiring serial flashing
3. **Tasmota firmware** - download from [tasmota.github.io](https://tasmota.github.io/docs/Download/)

#### Flashing Methods

**Method 1: Tasmota Web Installer (Easiest)**

For newer devices or those with existing Tuya firmware:

1. Open [https://tasmota.github.io/install/](https://tasmota.github.io/install/) in Chrome or Edge
2. Connect your device via USB (if supported)
3. Click "Connect" and select your device
4. Click "Install Tasmota" and wait for completion

**Method 2: Tuya Convert (Over-the-Air)**

For unmodified Tuya-based devices:

1. Set up Tuya Convert on a Raspberry Pi or Linux machine
2. Put your device in pairing mode (usually long-press the button)
3. Follow the Tuya Convert prompts to flash Tasmota

> **Warning:** Newer devices may have patched firmware that blocks Tuya Convert.

**Method 3: Serial Flashing (Most Reliable)**

For devices requiring hardware modification:

1. Open the device (void warranty)
2. Identify TX, RX, GND, 3.3V, and GPIO0 pins
3. Connect your USB-to-serial adapter:
   - TX → RX
   - RX → TX
   - GND → GND
   - 3.3V → 3.3V
4. Ground GPIO0 to enter flash mode
5. Use [Tasmotizer](https://github.com/tasmota/tasmotizer) to flash

### Configuring Tasmota for RustRide

After flashing, configure your Tasmota device for MQTT control.

#### Step 1: Initial WiFi Setup

1. Connect to the Tasmota device's WiFi access point (e.g., "tasmota-XXXX")
2. Your device should automatically open a configuration page
3. Enter your home WiFi credentials
4. Save and wait for the device to connect to your network

#### Step 2: Find the Device IP

1. Check your router's DHCP client list
2. Or use a network scanner app
3. Access the Tasmota web interface at `http://<device-ip>`

#### Step 3: Configure MQTT Settings

In the Tasmota web interface:

1. Go to **Configuration** > **Configure MQTT**
2. Configure these settings:

| Setting | Value | Description |
|---------|-------|-------------|
| **Host** | Your MQTT broker IP | e.g., `192.168.1.100` |
| **Port** | `1883` | Standard MQTT port |
| **User** | Your MQTT username | e.g., `rustride` |
| **Password** | Your MQTT password | Same as broker config |
| **Topic** | `training_fan` | Unique name for this device |
| **Full Topic** | `%prefix%/%topic%/` | Leave default |

3. Click **Save**
4. The device will restart and connect to your MQTT broker

#### Step 4: Verify MQTT Connection

In the Tasmota web console (main page > Console), you should see:

```
MQT: Connecting to 192.168.1.100:1883...
MQT: Connected
```

Test with mosquitto_sub:

```bash
# Subscribe to all Tasmota messages for your device
mosquitto_sub -h your-broker -t "stat/training_fan/#" -v
```

### Configuration by Device Type

#### On/Off Smart Plugs

For simple on/off plugs controlling a regular fan:

**Tasmota Topics:**
- Command: `cmnd/training_fan/POWER`
- Status: `stat/training_fan/POWER`

**RustRide Configuration:**

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `cmnd/training_fan/POWER` |
| **Add /set Suffix** | No |
| **Payload Format** | Custom |

**Zone Speed Mapping (On/Off Only):**

Since on/off plugs can't vary speed, configure zones to turn fan on/off at a threshold:

| Zone | Speed Setting | Effect |
|------|---------------|--------|
| Zone 1 | 0% | Fan OFF |
| Zone 2 | 0% | Fan OFF |
| Zone 3 | 100% | Fan ON |
| Zone 4 | 100% | Fan ON |
| Zone 5 | 100% | Fan ON |
| Zone 6 | 100% | Fan ON |
| Zone 7 | 100% | Fan ON |

**Tasmota Rules (Optional):**

Create a rule to handle numeric payloads:

```
Rule1 ON Dimmer#Data>0 DO Power 1 ENDON ON Dimmer#Data==0 DO Power 0 ENDON
Rule1 1
```

This allows RustRide to send speed values (0-100) and have the plug turn on for any value > 0.

#### Dimmer Plugs and Switches

For dimmers that provide true variable speed control:

**Tasmota Topics:**
- Command: `cmnd/training_fan/Dimmer`
- Status: `stat/training_fan/RESULT`

**RustRide Configuration:**

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `cmnd/training_fan/Dimmer` |
| **Add /set Suffix** | No |
| **Payload Format** | Speed Only |

**Testing:**

In Tasmota console, test the dimmer:

```
Dimmer 50
```

The device should set to 50% brightness/power.

Verify via MQTT:

```bash
mosquitto_pub -h your-broker -t "cmnd/training_fan/Dimmer" -m "75"
```

**Zone Speed Mapping (Variable):**

| Zone | Speed | Fan Level |
|------|-------|-----------|
| Zone 1 | 0% | Off |
| Zone 2 | 20% | Gentle breeze |
| Zone 3 | 40% | Low |
| Zone 4 | 55% | Medium |
| Zone 5 | 70% | High |
| Zone 6 | 85% | Very High |
| Zone 7 | 100% | Maximum |

#### PWM Controllers

For dedicated PWM fan control (12V/24V fans):

**Tasmota Configuration:**

1. In Tasmota console, configure PWM:
   ```
   SetOption15 1
   ```
   This enables PWM control via Dimmer commands.

2. Set PWM frequency for smooth fan operation:
   ```
   PwmFrequency 25000
   ```
   (25kHz is ideal for most PC fans)

**RustRide Configuration:**

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `cmnd/training_fan/Dimmer` |
| **Add /set Suffix** | No |
| **Payload Format** | Speed Only |

**Alternative: Direct PWM Control**

For more granular control:

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `cmnd/training_fan/PWM1` |
| **Add /set Suffix** | No |
| **Payload Format** | Speed Only |

PWM values 0-1023 correspond to 0-100% duty cycle.

### Alternative: Shelly Devices

Shelly devices offer another excellent option for smart fan control. They come with built-in MQTT support without requiring reflashing.

#### Shelly Plug S / Shelly Plug US

**Enable MQTT:**

1. Access Shelly web interface at device IP
2. Go to **Internet & Security** > **Advanced - Developer Settings**
3. Enable **MQTT**
4. Configure broker settings:
   - Server: `your-broker-ip:1883`
   - Username/Password: Your MQTT credentials

**RustRide Configuration:**

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `shellies/shellyplug-s-XXXXXX/relay/0/command` |
| **Add /set Suffix** | No |
| **Payload Format** | Custom |

**Payload mapping:**
- `on` = Turn on
- `off` = Turn off

> **Note:** Basic Shelly plugs only support on/off control. Use Shelly Dimmer for variable speed.

#### Shelly Dimmer 2

For variable speed fan control:

**RustRide Configuration:**

| Setting | Value |
|---------|-------|
| **MQTT Topic** | `shellies/shellydimmer2-XXXXXX/light/0/set` |
| **Add /set Suffix** | No |
| **Payload Format** | JSON Speed + On/Off |

**Payload format:**
```json
{"brightness": 75, "turn": "on"}
```

### Alternative: Tuya/Smart Life Devices with LocalTuya

If you have Tuya devices and don't want to flash custom firmware, you can use LocalTuya integration with Home Assistant to bridge to MQTT.

**Setup Steps:**

1. Install LocalTuya in Home Assistant
2. Add your Tuya devices locally (requires device ID and local key)
3. Create MQTT automation in Home Assistant (see Home Assistant section)
4. Configure RustRide to send commands through Home Assistant

### Multi-Fan Setups with Tasmota

For controlling multiple fans with a single Tasmota device:

#### Sonoff 4CH Pro Setup

Control up to 4 fans independently:

**Topics for each channel:**
- Fan 1: `cmnd/training_fans/POWER1`
- Fan 2: `cmnd/training_fans/POWER2`
- Fan 3: `cmnd/training_fans/POWER3`
- Fan 4: `cmnd/training_fans/POWER4`

**RustRide Configuration:**

Create a separate fan profile for each fan, with different zones triggering different channels:

**Profile: "Main Fan"**
- Topic: `cmnd/training_fans/POWER1`

**Profile: "Secondary Fan"**
- Topic: `cmnd/training_fans/POWER2`

**Tasmota Rule for Synchronized Control:**

To control all fans together with one command:

```
Rule1 ON Dimmer#Data DO Backlog Power1 %value% ; Power2 %value% ; Power3 %value% ENDON
Rule1 1
```

### Tasmota Console Commands Reference

Useful Tasmota commands for fan control setup:

| Command | Description | Example |
|---------|-------------|---------|
| `Status` | Show device status | `Status` |
| `Status 5` | Show network status | `Status 5` |
| `Status 6` | Show MQTT status | `Status 6` |
| `Power` | Toggle power | `Power` |
| `Power ON` | Turn on | `Power ON` |
| `Power OFF` | Turn off | `Power OFF` |
| `Dimmer` | Show current dimmer level | `Dimmer` |
| `Dimmer 50` | Set dimmer to 50% | `Dimmer 50` |
| `Topic` | Show current topic | `Topic` |
| `Topic newtopic` | Set new topic | `Topic training_fan` |
| `Restart 1` | Restart device | `Restart 1` |

### Troubleshooting Tasmota Integration

#### Device Not Connecting to MQTT

1. **Check WiFi connection:**
   ```
   Status 5
   ```
   Verify the device has an IP address.

2. **Check MQTT configuration:**
   ```
   Status 6
   ```
   Look for "MQTT Connected" status.

3. **Verify broker address:**
   - Ensure the broker IP is reachable from the device
   - Try using the broker's hostname vs. IP

4. **Check credentials:**
   - Verify username and password match your broker configuration
   - Check Mosquitto logs for authentication failures

#### Fan Not Responding to Commands

1. **Test directly in Tasmota console:**
   ```
   Dimmer 50
   ```
   If the device responds, the issue is MQTT configuration.

2. **Monitor MQTT traffic:**
   ```bash
   mosquitto_sub -h your-broker -t "cmnd/training_fan/#" -v
   ```
   Verify messages are being received.

3. **Check topic spelling:**
   - Topics are case-sensitive
   - No trailing slashes

4. **Verify payload format:**
   - Tasmota expects numeric values for Dimmer (0-100)
   - Use "Speed Only" payload format in RustRide

#### Dimmer Not Working (Always On/Off)

1. **Verify device supports dimming:**
   - Some devices are relay-only (on/off)
   - Check the Tasmota template for your device

2. **Enable dimmer functionality:**
   ```
   SetOption15 1
   ```

3. **Check minimum brightness:**
   - Some devices have a minimum dimmer level
   - Try setting a minimum zone speed of 10-15%

#### WiFi Disconnection Issues

1. **Enable WiFi fast reconnect:**
   ```
   SetOption56 1
   ```

2. **Set a static IP:**
   In Tasmota web UI: Configuration > Configure WiFi > Static IP

3. **Check WiFi signal strength:**
   ```
   Status 5
   ```
   Look for RSSI value (should be > -70 dBm)

#### Device Reboots or Crashes

1. **Check power supply:**
   - Ensure adequate power for your device
   - Some fans draw significant startup current

2. **Update Tasmota firmware:**
   - Check for latest stable release
   - Use OTA update in web UI

3. **Check for overcurrent:**
   - Verify your fan's power draw is within device limits

---

## Advanced: Custom Fan Devices

### DIY ESP8266/ESP32 Fan Controller

If you're building your own fan controller, use this MQTT message format:

**Subscribe to:** `home/fan/your_fan/set`

**Expected payloads:**

```json
{"speed": 0}      // Off
{"speed": 50}     // 50% speed
{"speed": 100}    // Full speed
```

**Publish status to (optional):** `home/fan/your_fan/state`

### Tasmota Smart Plugs with Dimmers

For Tasmota devices with dimmer capability:

1. Set up the topic in Tasmota console: `Topic rustride_fan`
2. In RustRide, use topic: `cmnd/rustride_fan/Dimmer`
3. Use payload format: **Speed Only**
4. Disable "Add /set suffix"

### PWM Fan Control via Tasmota

For PWM fan speed control:

1. Configure PWM on the Tasmota device
2. Use topic: `cmnd/your_device/pwm1`
3. Set payload format: **Speed Only**
4. Speed values 0-100 map to PWM duty cycle

---

## Related Documentation

- **Module Documentation:** `src/integrations/mqtt/` - Rust API documentation
- **Settings UI:** `src/ui/screens/settings.rs` - Settings screen implementation
- **Fan Controller:** `src/integrations/mqtt/fan.rs` - Fan control logic

---

## Changelog

### Initial Release

- MQTT broker connection with rumqttc
- Automatic reconnection on connection loss
- TLS/SSL support for secure connections
- Secure password storage in OS keyring
- Zone-based automatic fan speed control
- Manual fan speed override during rides
- Multiple fan profile support
- Support for multiple payload formats
- Test connection and test fan buttons
- Real-time connection status indicator
