# RustRide

A cross-platform indoor cycling application built in Rust with real-time sensor integration, structured workouts, and comprehensive ride analytics.

## Features

- **Bluetooth Smart Trainer Support** - Connect to FTMS-compatible smart trainers and power meters via Bluetooth LE
- **Real-time Metrics** - Live power, cadence, heart rate, and speed display with configurable smoothing
- **Training Zones** - Power and heart rate zones with visual indicators and time-in-zone tracking
- **Structured Workouts** - Import and execute workouts from ZWO (Zwift) and MRC/ERG formats
- **ERG Mode** - Automatic resistance control to match target power during workouts
- **Voice Alerts** - Text-to-speech announcements for interval changes, countdowns, and workout events
- **Ride Recording** - Automatic recording with pause detection and lap markers
- **Strava Sync** - Automatic upload to Strava with OAuth authentication, token refresh, and retry support
- **Export Formats** - Export rides to FIT, TCX, and CSV for upload to TrainingPeaks, Garmin Connect, etc.
- **Ride History** - Browse past rides with filtering, sorting, and detailed analytics
- **Offline-First** - All data stored locally in SQLite, no account required

## Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows | x64 | Supported |
| macOS | Intel (x64) | Supported |
| macOS | Apple Silicon (ARM64) | Supported |
| Linux | x64 | Supported |

## System Requirements

- **Windows**: Windows 10 or later, Bluetooth LE adapter
- **macOS**: macOS 11 (Big Sur) or later, built-in Bluetooth
- **Linux**: X11 or Wayland, BlueZ 5.x, Bluetooth LE adapter

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/ProvidenceIT/rust-ride/releases) page.

### From Source

Requires Rust 1.75 or later.

```bash
# Clone the repository
git clone https://github.com/ProvidenceIT/rust-ride.git
cd rust-ride

# Build release binary
cargo build --release

# Run
./target/release/rustride
```

#### Linux Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install libdbus-1-dev pkg-config libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev \
  libgtk-3-dev libatk1.0-dev libcairo2-dev libpango1.0-dev libgdk-pixbuf2.0-dev

# For voice alerts (text-to-speech)
sudo apt-get install speech-dispatcher libspeechd-dev
```

## Voice Alerts (Text-to-Speech)

RustRide includes voice announcements for hands-free workout guidance. Voice alerts announce:
- Interval changes with target power and duration
- Countdown warnings before interval transitions (10, 5, 3, 2, 1 seconds)
- Workout start, pause, resume, and completion
- Optional motivational cues during high-intensity and recovery intervals

### Voice Alert Settings

Configure voice alerts in **Settings > Voice Alerts**:
- **Voice Selection** - Choose from available system voices
- **Speech Rate** - Adjust how fast announcements are spoken (0.5x - 2.0x)
- **Volume** - Control voice alert volume independently from sound effects
- **Per-Alert Configuration** - Enable/disable voice vs. sound for specific alert types

### Platform-Specific Requirements

#### Windows
Text-to-speech uses the built-in **Windows SAPI** (Speech API). This is available by default on Windows 10 and later.

**Troubleshooting:**
- If voices are not available, ensure text-to-speech is enabled in Windows Settings > Time & Language > Speech
- Additional voices can be installed through Windows Settings > Time & Language > Speech > Manage voices
- For PowerShell installation: `Add-WindowsCapability -Online -Name Language.Speech~~~en-US~0.0.1.0`

#### macOS
Text-to-speech uses **AVSpeechSynthesizer**, which is built into macOS 11 (Big Sur) and later.

**Troubleshooting:**
- Voices are managed in System Preferences > Accessibility > Spoken Content
- Additional voices can be downloaded through System Preferences > Accessibility > Spoken Content > System Voice > Manage Voices

#### Linux
Text-to-speech uses **speech-dispatcher**, which must be installed separately.

**Installation:**
```bash
# Ubuntu/Debian
sudo apt-get install speech-dispatcher speech-dispatcher-espeak-ng

# Fedora
sudo dnf install speech-dispatcher speech-dispatcher-espeak-ng

# Arch Linux
sudo pacman -S speech-dispatcher espeak-ng
```

**Troubleshooting:**
- Verify speech-dispatcher is running: `speech-dispatcher --version`
- Test speech from command line: `spd-say "Hello world"`
- If no audio, check that pulseaudio or pipewire is running
- Ensure your user is in the `audio` group: `sudo usermod -aG audio $USER`

### Known Limitations

- **Thread Safety**: TTS operations run on a dedicated worker thread to handle platform-specific constraints (especially on macOS where the TTS engine is not thread-safe)
- **Voice Availability**: Available voices depend on the system language packs installed
- **Linux Virtual Environments**: TTS may not work in Docker containers or VMs without proper audio passthrough configuration
- **Simultaneous Speech**: Only one voice announcement plays at a time; high-priority alerts (interval changes) interrupt lower-priority ones

## Quick Start

1. **Launch the application** - Run the `rustride` executable
2. **Connect sensors** - Go to Sensor Setup and scan for Bluetooth devices
3. **Configure FTP** - Set your Functional Threshold Power in Settings for accurate zone calculations
4. **Start riding** - Return to Home and click "Start Ride"

## Strava Integration

RustRide can automatically upload your rides to Strava after each session. To enable this feature, you need to set up Strava API credentials.

### Step 1: Create a Strava API Application

1. **Log in to Strava** - Visit [www.strava.com](https://www.strava.com) and sign in to your account
2. **Go to API Settings** - Navigate to [www.strava.com/settings/api](https://www.strava.com/settings/api)
3. **Create Your Application**:
   - **Application Name**: Enter a name (e.g., "RustRide")
   - **Category**: Select "Training"
   - **Club**: Leave blank (optional)
   - **Website**: Enter any valid URL (e.g., "https://github.com/ProvidenceIT/rust-ride")
   - **Application Description**: Brief description of your use
   - **Authorization Callback Domain**: Enter `localhost`
4. **Accept the API Agreement** and click "Create"

### Step 2: Get Your API Credentials

After creating your application, you'll see:
- **Client ID**: A numeric identifier (e.g., `12345`)
- **Client Secret**: A long alphanumeric string (keep this secure!)

> ⚠️ **Security Note**: Never share your Client Secret publicly. Treat it like a password.

### Step 3: Configure RustRide

Set the following environment variables before launching RustRide:

**Windows (PowerShell):**
```powershell
$env:STRAVA_CLIENT_ID = "your_client_id"
$env:STRAVA_CLIENT_SECRET = "your_client_secret"
.\rustride.exe
```

**Windows (Command Prompt):**
```cmd
set STRAVA_CLIENT_ID=your_client_id
set STRAVA_CLIENT_SECRET=your_client_secret
rustride.exe
```

**macOS / Linux:**
```bash
export STRAVA_CLIENT_ID="your_client_id"
export STRAVA_CLIENT_SECRET="your_client_secret"
./rustride
```

For persistent configuration, add these to your shell profile (`.bashrc`, `.zshrc`, or Windows environment variables).

### Step 4: Connect Your Account

1. Open RustRide and go to **Settings > Strava**
2. Click **"Connect to Strava"**
3. Your browser will open for Strava authorization
4. Log in and click **"Authorize"** to grant RustRide access
5. Return to RustRide - you should see your Strava profile

### Features

Once connected, RustRide provides:

- **Auto-sync**: Automatically upload rides after each session (configurable)
- **Manual retry**: Retry failed uploads from Ride History
- **Token refresh**: Automatic token refresh when expired
- **Upload status**: View sync status in Ride History and Ride Detail screens

### Troubleshooting

| Issue | Solution |
|-------|----------|
| "Authorization required" error | Ensure STRAVA_CLIENT_ID and STRAVA_CLIENT_SECRET are set correctly |
| Browser doesn't open | Check if the authorization URL is displayed in the console |
| Upload fails repeatedly | Verify your internet connection and Strava service status |
| "Token expired" after reconnect | Disconnect and reconnect your Strava account |

### API Rate Limits

Strava imposes rate limits on API usage:
- **100 requests per 15 minutes**
- **1,000 requests per day**

RustRide is designed to stay well within these limits for normal usage. If you encounter rate limiting, wait 15 minutes before retrying.

## Workout File Formats

### ZWO (Zwift Workouts)
Place `.zwo` files in the workouts directory. These are XML-based files with support for:
- Steady-state intervals
- Ramps (gradual power changes)
- Free ride sections
- Text instructions

### MRC/ERG (TrainerRoad/Golden Cheetah)
Place `.mrc` or `.erg` files in the workouts directory. These are text-based files with:
- Time and power percentage pairs
- Course header information

## Data Storage

All data is stored locally:
- **Database**: `~/.rustride/rustride.db` (SQLite)
- **Config**: `~/.rustride/config.toml`
- **Workouts**: `~/.rustride/workouts/`

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Build documentation
cargo doc --no-deps --open
```

## Architecture

```
src/
├── app.rs              # Main application state and event loop
├── main.rs             # Entry point
├── lib.rs              # Library exports
├── audio/              # Audio alerts and text-to-speech
│   ├── tts.rs          # Cross-platform TTS provider (SAPI/AVSpeech/speech-dispatcher)
│   ├── engine.rs       # Audio engine with priority queue
│   ├── cues.rs         # Message templates for workout announcements
│   └── workout_bridge.rs # Bridges workout events to audio alerts
├── sensors/            # Bluetooth sensor management and FTMS parsing
├── metrics/            # Real-time metrics calculation and zones
├── recording/          # Ride recording and export (TCX, CSV)
├── workouts/           # Workout parsing (ZWO, MRC) and execution engine
├── storage/            # SQLite database and configuration
└── ui/                 # egui-based user interface
    ├── screens/        # Application screens (home, ride, settings, etc.)
    ├── widgets/        # Reusable UI components
    └── theme.rs        # Visual styling
```

## License

MIT License - see [LICENSE](LICENSE) for details.