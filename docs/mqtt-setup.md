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
