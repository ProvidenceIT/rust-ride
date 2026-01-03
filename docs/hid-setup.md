# HID Device Setup Guide

This document describes how to set up and configure USB HID devices (Stream Deck, foot pedals, button controllers) for use with RustRide. These devices allow hands-free control of your workouts.

## Overview

RustRide supports USB Human Interface Devices (HID) for physical button control of:

- Ride controls (pause/resume, lap markers, end ride)
- Workout controls (skip interval, extend interval, restart interval)
- Audio controls (volume up/down, mute)
- Fan controls (speed up/down, toggle)
- Navigation (switch between metrics, map, and workout views)

---

## Supported Devices

### Elgato Stream Deck Family

RustRide has first-class support for all Stream Deck models with automatic detection and sensible default button mappings.

| Device | Vendor ID | Product ID | Buttons | Default Mappings |
|--------|-----------|------------|---------|------------------|
| Stream Deck (Original) | 0x0FD9 | 0x0060 | 15 | 9 pre-configured |
| Stream Deck Mini | 0x0FD9 | 0x006C | 6 | 6 pre-configured |
| Stream Deck XL | 0x0FD9 | 0x006D | 32 | 14 pre-configured |
| Stream Deck MK.2 | 0x0FD9 | 0x0080 | 15 | 9 pre-configured |
| Stream Deck Pedal | 0x0FD9 | 0x0086 | 3 | 3 pre-configured |

#### Stream Deck Default Mappings

**Stream Deck (15 buttons):**
- Button 1 (top-left): Pause/Resume
- Button 2: Add Lap Marker
- Button 3: Skip Interval
- Button 6: Restart Interval
- Button 11: Volume Up
- Button 12: Volume Down
- Button 13: Mute Toggle
- Button 14: Fan Speed Up
- Button 15: Fan Speed Down

**Stream Deck Pedal (3 foot pedals):**
- Left Pedal: Add Lap Marker
- Center Pedal: Pause/Resume
- Right Pedal: Skip Interval

### Generic USB Button Controllers

RustRide also supports generic USB button controllers that use standard HID protocols.

| Device | Vendor ID | Product ID | Type | Buttons |
|--------|-----------|------------|------|---------|
| Generic USB Foot Pedal (Microdia) | 0x0C45 | 0x7403 | Keyboard | 3 |
| VEC USB Foot Pedal | 0x05F3 | 0x00FF | Keyboard | 3 |
| Olympus RS Foot Control | 0x07B4 | 0x0218 | Byte-per-button | 4 |

### Other HID Devices

Devices not in the known device list can still work with RustRide. The application will attempt to auto-detect the report format:

- **Gamepad-style devices**: Bitmap encoding (one bit per button)
- **Keyboard-style devices**: Standard HID keyboard scan codes
- **Media controllers**: Consumer control codes (play/pause, volume, etc.)

---

## Platform Setup

### Linux

On Linux, USB HID devices are typically only accessible to root by default. You need to configure udev rules to allow non-root access.

#### Step 1: Create udev Rules File

Create a file at `/etc/udev/rules.d/99-rustride-hid.rules`:

```bash
sudo nano /etc/udev/rules.d/99-rustride-hid.rules
```

Add the following rules:

```udev
# RustRide HID Device Rules
# Allow non-root access to supported HID devices

# Elgato Stream Deck (Original)
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0060", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0060", MODE="0666"

# Elgato Stream Deck Mini
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="006c", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="006c", MODE="0666"

# Elgato Stream Deck XL
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="006d", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="006d", MODE="0666"

# Elgato Stream Deck MK.2
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0080", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0080", MODE="0666"

# Elgato Stream Deck Pedal
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0086", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0086", MODE="0666"

# Generic USB Foot Pedal (Microdia)
SUBSYSTEM=="usb", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="7403", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="7403", MODE="0666"

# VEC USB Foot Pedal
SUBSYSTEM=="usb", ATTRS{idVendor}=="05f3", ATTRS{idProduct}=="00ff", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="05f3", ATTRS{idProduct}=="00ff", MODE="0666"

# Olympus RS Foot Control
SUBSYSTEM=="usb", ATTRS{idVendor}=="07b4", ATTRS{idProduct}=="0218", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="07b4", ATTRS{idProduct}=="0218", MODE="0666"
```

**Security Note:** `MODE="0666"` allows any user to access the device. For a more secure setup, you can use `GROUP="plugdev"` and add your user to the `plugdev` group:

```udev
# More secure alternative - only users in 'plugdev' group can access
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0060", MODE="0660", GROUP="plugdev"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0060", MODE="0660", GROUP="plugdev"
```

#### Step 2: Reload udev Rules

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

#### Step 3: Verify Access

Unplug and replug your device, then check permissions:

```bash
# Find your device
ls -la /dev/hidraw*

# Should show appropriate permissions
# crw-rw-rw- 1 root root ... /dev/hidraw0
```

#### Alternative: Add User to Input Group

Some distributions allow HID access through the `input` group:

```bash
sudo usermod -aG input $USER
# Log out and back in for changes to take effect
```

---

### Windows

Windows typically works out of the box for most HID devices. However, some Stream Deck models may require additional setup.

#### Stream Deck Devices

1. **Install Stream Deck Software** (optional but recommended)
   - Download from [Elgato's website](https://www.elgato.com/downloads)
   - The software installs the necessary USB drivers
   - You can close the Stream Deck software while using RustRide

2. **Alternative: WinUSB Driver**
   - If you don't want the official software, you can use [Zadig](https://zadig.akeo.ie/) to install WinUSB drivers
   - Run Zadig as Administrator
   - Select your Stream Deck from the device list
   - Install the WinUSB driver

#### Generic USB Devices

Most generic USB foot pedals and button controllers work without additional drivers on Windows as they use standard HID protocols.

#### Troubleshooting Driver Issues

If your device isn't detected:

1. Open Device Manager (`devmgmt.msc`)
2. Look for your device under "Human Interface Devices" or "USB Input Device"
3. If showing an error, right-click and select "Update driver"
4. Choose "Search automatically for drivers"

---

### macOS

macOS requires explicit user permission to access HID devices.

#### Step 1: Grant Input Monitoring Permission

1. Open **System Preferences** (or System Settings on macOS Ventura+)
2. Go to **Security & Privacy** > **Privacy**
3. Select **Input Monitoring** from the left sidebar
4. Click the lock icon to make changes
5. Add RustRide to the allowed applications list
6. Restart RustRide after granting permission

#### Step 2: Grant Accessibility Access (if needed)

Some HID devices may also require Accessibility access:

1. In **Security & Privacy** > **Privacy**
2. Select **Accessibility**
3. Add RustRide to the allowed applications list

#### Stream Deck on macOS

Stream Deck devices may be claimed by the official Stream Deck software. If you have the Stream Deck software installed:

1. Open the Stream Deck software
2. Go to Preferences
3. Disable "Launch at startup" if you prefer RustRide to control the device
4. Quit the Stream Deck software before starting RustRide

**Note:** Only one application can access a Stream Deck at a time.

---

## Configuration in RustRide

### Scanning for Devices

1. Open RustRide
2. Go to **Settings** > **Hardware** > **HID Devices**
3. Click **Scan for Devices**
4. Detected devices will appear in the list

### Enabling a Device

1. Locate your device in the list
2. Toggle the **Enabled** switch
3. The device status should change to "Connected"

### Configuring Button Mappings

1. Select a device from the list
2. Click **Edit Mappings**
3. To add a new mapping:
   - Click **Add Button Mapping**
   - Press the button on your device (learning mode)
   - Select the action to assign
   - Click **Save**
4. To remove a mapping, click the delete icon next to it

### Available Actions

| Category | Action | Description |
|----------|--------|-------------|
| **Ride** | Pause/Resume | Toggle ride pause state |
| **Ride** | Add Lap Marker | Mark a lap in your ride |
| **Ride** | End Ride | End and save the current ride |
| **Workout** | Skip Interval | Skip to the next workout interval |
| **Workout** | Extend Interval | Add 30 seconds to current interval |
| **Workout** | Restart Interval | Restart the current interval |
| **Audio** | Volume Up | Increase audio volume by 10% |
| **Audio** | Volume Down | Decrease audio volume by 10% |
| **Audio** | Mute Toggle | Toggle audio mute |
| **Fan** | Fan Speed Up | Increase fan speed by 10% |
| **Fan** | Fan Speed Down | Decrease fan speed by 10% |
| **Fan** | Fan Toggle | Turn fan on/off |
| **Navigation** | Show Metrics | Switch to metrics view |
| **Navigation** | Show Map | Switch to map view |
| **Navigation** | Show Workout | Switch to workout view |
| **Navigation** | Toggle Fullscreen | Toggle fullscreen mode |

---

## Auto-Reconnect

RustRide automatically reconnects to devices that are unplugged and replugged. This is useful if:

- You accidentally disconnect a USB cable
- Your USB hub loses power briefly
- You need to swap devices during a ride

### How It Works

1. When you open a device, RustRide remembers it
2. If the device is disconnected, RustRide monitors for it
3. When reconnected, the device is automatically reopened
4. All button mappings are restored

### Configuration

Auto-reconnect is enabled by default. You can configure:

- **Enable/Disable**: Toggle auto-reconnect in HID settings
- **Reconnect Delay**: Time to wait before reconnecting (default: 1 second)

---

## Troubleshooting

### Device Not Detected

**Symptoms:** Device doesn't appear after scanning

**Solutions:**

1. **Check USB connection**
   - Try a different USB port
   - Avoid USB hubs if possible
   - Use the cable that came with your device

2. **Check platform permissions**
   - Linux: Verify udev rules are in place and reloaded
   - macOS: Verify Input Monitoring permission is granted
   - Windows: Check Device Manager for driver issues

3. **Check for conflicting software**
   - Stream Deck software may claim the device exclusively
   - Close other applications that might use the device

4. **Restart RustRide**
   - Sometimes a restart helps after permission changes

### Device Shows "Permission Denied"

**Linux:**
```bash
# Check current permissions
ls -la /dev/hidraw*

# If owned by root with no group access, update udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Unplug and replug the device
```

**macOS:**
- Ensure RustRide has Input Monitoring permission
- Try adding Accessibility permission as well
- Restart the application after granting permissions

**Windows:**
- Run RustRide as Administrator (temporary fix)
- Check if antivirus is blocking device access

### Button Presses Not Registering

**Symptoms:** Device connected but button presses have no effect

**Solutions:**

1. **Check button mappings**
   - Ensure mappings exist for the buttons you're pressing
   - Use learning mode to capture the correct button codes

2. **Verify device is enabled**
   - Check the enabled toggle in settings

3. **Check action context**
   - Some actions only work in specific contexts:
     - Ride actions require an active ride
     - Workout actions require an active workout
     - Fan actions require a configured fan profile

4. **Check for report format issues**
   - For generic devices, the auto-detected format may be wrong
   - Try disconnecting and reconnecting the device

### Stream Deck Buttons Mapped Wrong

Stream Deck buttons are numbered 0-14 (or 0-5 for Mini, 0-31 for XL). If buttons seem mapped incorrectly:

1. Use learning mode to capture the actual button code
2. Create custom mappings instead of relying on defaults
3. Button numbering is row-major (left-to-right, top-to-bottom)

**Stream Deck Layout (15 buttons):**
```
[ 0] [ 1] [ 2] [ 3] [ 4]
[ 5] [ 6] [ 7] [ 8] [ 9]
[10] [11] [12] [13] [14]
```

### Device Keeps Disconnecting

**Symptoms:** Device repeatedly connects and disconnects

**Solutions:**

1. **USB power issues**
   - Use a powered USB hub
   - Try a different USB port
   - Check cable quality

2. **USB suspend issues (Linux)**
   ```bash
   # Disable USB autosuspend for the device
   echo -1 | sudo tee /sys/bus/usb/devices/*/power/autosuspend
   ```

3. **Windows power management**
   - Open Device Manager
   - Find your USB hub
   - Right-click > Properties > Power Management
   - Uncheck "Allow the computer to turn off this device"

### Latency Issues

**Symptoms:** Noticeable delay between button press and action

**Solutions:**

1. **Close unnecessary applications**
2. **Use a direct USB connection** (avoid hubs)
3. **Check CPU usage** during workouts
4. **Reduce video quality** if using 3D world

---

## Finding Device IDs

If you have an unsupported device and want to request support, you'll need the Vendor ID and Product ID.

### Linux

```bash
lsusb
# Example output:
# Bus 001 Device 005: ID 0fd9:0060 Elgato Systems GmbH Stream Deck
#                        ^^^^ ^^^^
#                        VID  PID
```

### Windows

1. Open Device Manager
2. Find your device under "Human Interface Devices"
3. Right-click > Properties > Details
4. Select "Hardware Ids" from the dropdown
5. Look for `VID_XXXX&PID_XXXX`

### macOS

```bash
system_profiler SPUSBDataType | grep -A 10 "Stream Deck"
# Look for "Vendor ID" and "Product ID"
```

---

## Related Documentation

- **Module Documentation:** `src/hid/mod.rs` - Rust API documentation
- **Settings UI:** `src/ui/screens/settings.rs` - Settings screen implementation
- **Actions Reference:** `src/hid/actions.rs` - All available button actions

---

## Changelog

### Initial Release

- Support for all Stream Deck models (Original, Mini, XL, MK.2, Pedal)
- Support for generic USB foot pedals and button controllers
- Auto-detect for gamepad, keyboard, and consumer control formats
- Automatic device reconnection
- Configurable button mappings
- Linux udev rules documentation
- Windows driver guidance
- macOS permission instructions
