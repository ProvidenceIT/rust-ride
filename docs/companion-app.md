# RustRide Mobile Companion App

Control your indoor training sessions from your phone with the RustRide Companion App. View real-time metrics, control workouts, and browse your ride history - all wirelessly connected to the desktop app.

## Overview

The Mobile Companion App allows you to:

- **View Live Metrics** - See power, heart rate, cadence, speed, and distance in real-time
- **Control Workouts** - Pause, resume, skip intervals, and stop sessions from your phone
- **Browse History** - View past rides with detailed statistics
- **Adjust Resistance** - Change trainer resistance during free rides

The companion connects to RustRide over your local network using WebSocket for low-latency communication. mDNS service discovery makes finding and connecting to your desktop automatic.

---

## Supported Platforms

| Platform | Status |
|----------|--------|
| iOS | Supported (iOS 14+) |
| Android | Supported (Android 10+) |

---

## Getting Started

### Prerequisites

Before using the companion app:

1. **Same Network** - Your phone and computer must be on the same local network (Wi-Fi)
2. **RustRide Running** - The desktop app must be running with the companion server enabled
3. **Firewall** - Ensure your firewall allows connections on port 9876 (or your configured port)

### Step 1: Enable Companion Server

1. Open RustRide on your desktop
2. Go to **Settings > Companion App**
3. Enable the **"Enable companion server"** toggle
4. Note the displayed port (default: 9876)
5. Optionally enable **"Require PIN"** for security

When the server starts successfully, you'll see:
- A green "Running" status indicator
- A 6-digit PIN (if PIN authentication is enabled)
- A QR code for easy mobile pairing

### Step 2: Install the Mobile App

Download the RustRide Companion app for your platform:

- **iOS**: Available on the App Store
- **Android**: Available on Google Play

### Step 3: Connect to RustRide

There are three ways to connect your phone to the desktop app:

#### Option A: Automatic Discovery (Recommended)

1. Open the companion app on your phone
2. Go to the **Connection** tab
3. Your RustRide desktop should appear in the list automatically
4. Tap on your computer to connect
5. Enter the PIN if prompted

#### Option B: QR Code Pairing

1. On the desktop app, go to **Settings > Companion App**
2. Find the QR code in the "Quick Pairing" section
3. On your phone, tap the **Scan QR** button
4. Point your camera at the QR code
5. The app will connect automatically

The QR code contains:
- Connection URL (`ws://your-ip:port`)
- PIN (if required)
- Protocol version

#### Option C: Manual Entry

If automatic discovery doesn't work:

1. On the companion app, tap **"Enter Manually"**
2. Enter your computer's IP address and port (e.g., `192.168.1.100:9876`)
3. Tap **Connect**
4. Enter the PIN if prompted

To find your IP address:
- **Windows**: Run `ipconfig` in Command Prompt, look for IPv4 Address
- **macOS**: System Settings > Network > Wi-Fi > Details, look for IP Address
- **Linux**: Run `ip addr` or `hostname -I`

---

## Features

### Dashboard

The Dashboard shows real-time metrics during your ride:

| Metric | Description |
|--------|-------------|
| **Power** | Current power output in watts with 3-second average and power zone indicator (Z1-Z7) |
| **Heart Rate** | Current HR in BPM with HR zone indicator and session max |
| **Cadence** | Current pedaling speed in RPM with target indicator for structured workouts |
| **Speed** | Current speed in km/h or mph (based on your unit preference) |
| **Distance** | Total distance covered in the session |
| **Elapsed Time** | Session duration in HH:MM:SS format |
| **Calories** | Estimated calories burned |

**During Workouts:**
- Current interval name and progress bar
- Time remaining in the interval
- Target power display
- Next interval preview

### Workout Controls

When a session is active, the control bar provides:

| Button | Action |
|--------|--------|
| **Play/Pause** | Pause or resume the workout |
| **Skip** | Skip to the next interval (workouts only) |
| **Stop** | End the session (with confirmation) |

For free rides, you can also adjust the trainer resistance using +/- buttons.

### Ride History

Browse your past rides with:

- **Summary List** - Date, duration, distance, average power
- **Filtering** - Filter by date range (week, month, year) or ride type
- **Detailed Stats** - Tap a ride for full statistics:
  - Power: Average, Max, Normalized Power (NP), Intensity Factor (IF)
  - Heart Rate: Average and Max
  - Training Load: TSS (Training Stress Score)
  - Calories burned

### Settings

Configure your companion app preferences:

| Setting | Description |
|---------|-------------|
| **Units** | Metric (km, km/h) or Imperial (mi, mph) |
| **Theme** | Light, Dark, or System default |
| **Keep Screen Awake** | Prevent screen from sleeping during active sessions |
| **Haptic Feedback** | Vibration intensity for button presses and interval changes (Off, Light, Medium, Strong) |
| **Auto-Reconnect** | Automatically reconnect to last server on app launch |
| **Remember PIN** | Securely store the PIN for faster reconnection |

---

## Connection Troubleshooting

### "No servers found"

**Possible Causes:**
1. Desktop app is not running
2. Companion server is not enabled
3. Devices are on different networks
4. mDNS is blocked by network/firewall

**Solutions:**
- Verify RustRide is running and go to Settings > Companion App
- Ensure "Enable companion server" is toggled on
- Check both devices are on the same Wi-Fi network
- Try manual IP entry instead of discovery
- Check firewall settings (see below)

### "Connection failed"

**Possible Causes:**
1. Firewall blocking the connection
2. Incorrect IP address or port
3. Port already in use by another application

**Solutions:**
- Allow RustRide through your firewall on port 9876
- Verify the IP address matches your computer
- Try a different port in desktop Settings > Companion App
- Restart the RustRide desktop app

### "Authentication failed" / Wrong PIN

**Possible Causes:**
1. Incorrect PIN entered
2. PIN was regenerated on desktop
3. Connection timeout

**Solutions:**
- Check the PIN displayed in Settings > Companion App on desktop
- Use QR code pairing for automatic PIN entry
- Regenerate PIN on desktop and try again
- Disable "Require PIN" if on a trusted home network

### Connection drops frequently

**Possible Causes:**
1. Weak Wi-Fi signal
2. Network congestion
3. Phone entering sleep mode

**Solutions:**
- Move closer to your Wi-Fi router
- Reduce network load from other devices
- Enable "Keep Screen Awake" in app settings
- Disable battery optimization for the companion app

### Metrics not updating

**Possible Causes:**
1. No active session on desktop
2. Sensors not connected to desktop
3. Subscription not active

**Solutions:**
- Start a ride or workout on the desktop app
- Check sensor connections in desktop app
- Disconnect and reconnect from companion app
- Restart both apps

---

## Firewall Configuration

### Windows

1. Open **Windows Defender Firewall with Advanced Security**
2. Click **Inbound Rules** > **New Rule**
3. Select **Port** and click Next
4. Select **TCP** and enter port **9876**
5. Select **Allow the connection**
6. Check all network types (Domain, Private, Public)
7. Name the rule "RustRide Companion"

Or using PowerShell (run as Administrator):
```powershell
New-NetFirewallRule -DisplayName "RustRide Companion" -Direction Inbound -Protocol TCP -LocalPort 9876 -Action Allow
```

### macOS

1. Go to **System Settings > Network > Firewall**
2. Click **Options**
3. Add RustRide to allowed applications
4. Or disable the firewall for testing

Or using Terminal:
```bash
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /Applications/RustRide.app
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --unblockapp /Applications/RustRide.app
```

### Linux

Using UFW:
```bash
sudo ufw allow 9876/tcp comment "RustRide Companion"
```

Using firewalld:
```bash
sudo firewall-cmd --permanent --add-port=9876/tcp
sudo firewall-cmd --reload
```

Using iptables:
```bash
sudo iptables -A INPUT -p tcp --dport 9876 -j ACCEPT
```

---

## Settings Reference

### Desktop Companion Settings

Located in **Settings > Companion App**:

| Setting | Default | Description |
|---------|---------|-------------|
| Enable companion server | Off | Toggle to start/stop the server |
| Port | 9876 | TCP port for WebSocket connections |
| Require PIN | On | Require 6-digit PIN for authentication |
| Session timeout | 1 hour | Auto-disconnect idle connections (0 = never) |
| Max connections | 5 | Maximum simultaneous companion connections |

### Mobile App Settings

Located in the **Settings** tab:

| Setting | Default | Description |
|---------|---------|-------------|
| Units | Metric | Measurement units (Metric/Imperial) |
| Theme | System | Color theme (Light/Dark/System) |
| Keep Screen Awake | On | Prevent screen sleep during active sessions |
| Haptic Feedback | Medium | Vibration intensity for feedback |
| Auto-Reconnect | On | Auto-connect to last server on launch |
| Remember PIN | Off | Store PIN securely for quick reconnection |

---

## Security Considerations

### PIN Authentication

When enabled, PIN authentication:
- Uses a 6-digit numeric PIN generated by the desktop app
- Must be entered within 60 seconds of connection
- Locks out after 5 failed attempts (10-minute lockout)
- PINs can be regenerated from the desktop app at any time

**Recommendations:**
- Enable PIN on public or shared networks
- Can be disabled on trusted home networks for convenience
- Never share your PIN with untrusted parties

### Local Network Only

The companion server:
- Only accepts connections from your local network
- Does not expose any ports to the internet
- All communication stays within your LAN

### Secure PIN Storage

When "Remember PIN" is enabled on mobile:
- PIN is stored in the device's secure keychain/keystore
- Encrypted at rest using platform security
- Never transmitted or logged in plain text

---

## Known Limitations

- **Single Desktop**: Can only connect to one desktop at a time
- **No Internet Required**: Works entirely on local network; no cloud services
- **Desktop Must Be Running**: Cannot start sessions from mobile - only control existing ones
- **Sensor Data**: All sensors must be connected to the desktop app

---

## Changelog

### Version 1.0 (Initial Release)

- Real-time metrics streaming at 1Hz
- Workout controls (pause, resume, skip, stop)
- Ride history with detailed statistics
- mDNS auto-discovery
- QR code pairing
- PIN authentication
- Dark/light theme support
- Metric/imperial units
- Haptic feedback options
- Offline ride cache

---

## Developer Resources

For developers building custom integrations or troubleshooting advanced issues:

- **[Companion API Reference](./companion-api.md)** - Complete WebSocket API documentation with message types, authentication flow, and JSON schemas
