# RustRide Feature Roadmap

A comprehensive list of features for future development, organized by category and priority.

---

## Table of Contents

1. [Training Science & Analytics](#1-training-science--analytics)
2. [AI & Machine Learning](#2-ai--machine-learning)
3. [3D World & Content](#3-3d-world--content)
4. [Social & Multiplayer](#4-social--multiplayer)
5. [Hardware Integration](#5-hardware-integration)
6. [UX & Accessibility](#6-ux--accessibility)
7. [Platform & Technical](#7-platform--technical)
8. [Competitive Feature Gaps](#8-competitive-feature-gaps)

---

## 1. Training Science & Analytics

Advanced training metrics and periodization features based on sports science research.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Critical Power (CP) / W' Model** | Two-parameter power model for precise pacing. W' = anaerobic reserve in kJ. Enables time predictions at any power target. | Medium |
| **FTP Auto-Detection** | AI-powered FTP estimation from workout data without dedicated testing. 38% less overestimation vs traditional tests. | Medium |
| **Power Duration Curve (PDC)** | Plot max power across all durations (5s to hours). Identifies strengths/weaknesses. Classifies rider type. | Medium |
| **Normalized Power / TSS / IF Dashboard** | Rolling average power calculations, Training Stress Score, Intensity Factor for accurate workout quantification. | Medium |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Acute:Chronic Workload Ratio (ACWR)** | 7-day/28-day load ratio. Sweet spot 0.8-1.3 minimizes injury risk. Alerts for dangerous spikes. | Medium |
| **HRV Training Readiness** | Morning HRV measurement for recovery status. Traffic-light guidance (green/amber/red). | High |
| **Periodization Builder** | Automated macrocycle/mesocycle/microcycle planning. Base→Build→Peak→Recovery phases. | Medium-High |
| **Aerobic Threshold (LT1) / Two-Threshold Model** | Dual threshold training: LT1 (~2mmol/L) for base, LT2 (FTP) for power. | High |
| **VO2max Prediction** | Estimate VO2max from 5-min max test. Program Zone 5 intervals at 106-120% FTP. | Medium |
| **Sweet Spot Training Optimizer** | Automated 88-93% FTP workouts. Optimal duration/frequency recommendations. | Low-Medium |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **dFRC Real-Time Tracking** | Live anaerobic capacity depletion/recovery during workouts. | High |
| **Race Simulation Builder** | Course-specific simulations with variable pacing strategies. | Medium-High |
| **Stamina/Fatigue Index** | Track power maintenance across ride duration. Predict performance degradation. | High |
| **Sleep Integration** | Wearable sync for sleep quality → recovery-based training adjustments. | High |

---

## 2. AI & Machine Learning

Machine learning features for personalization and intelligent coaching.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **AI FTP Prediction** | ML model trained on workout patterns. No manual testing required. | Medium |
| **Adaptive Workout Recommendations** | Collaborative filtering + content-based recommendations. Considers fatigue/goals. | Medium-High |
| **Real-Time Fatigue Detection** | HRV analysis, aerobic decoupling, isolation forest anomaly detection. 85-91% accuracy. | Medium |
| **Performance Trend Forecasting** | 4-12 week fitness projections. Plateau detection. Detraining alerts. | High |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Workout Difficulty Estimation** | Predict difficulty from workout structure. Real-time adjustment during rides. | Low-Medium |
| **Power Curve Profiling** | Auto-classify rider type: sprinter, climber, all-rounder, time trialist. | Low-Medium |
| **Cadence & Technique Analysis** | Optimal cadence detection. Efficiency metrics. Technique degradation alerts. | Medium |
| **Training Load Adaptation Engine** | Gradient boosting for personalized TSS optimization. Prevent overtraining. | High |
| **Periodization Phase Detection** | Auto-detect current training phase from workout patterns. | Medium |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Race Performance Prediction** | Finish time projections based on power profile + course data. | High |
| **Equipment Health Monitoring** | Autoencoder/isolation forest for sensor drift/failure detection. | Medium |
| **Injury Risk Assessment** | Classification model for training-related injury prediction. | Medium-High |
| **Aerobic Fitness Classification** | Responder vs non-responder identification. 87-93% accuracy. | Medium |
| **Nutrition Impact Analysis** | Correlate fueling with performance outcomes. | Low-Medium |
| **Motivation Modeling** | Identify motivation archetype. Prevent engagement slumps. | Medium |

---

## 3. 3D World & Content

Virtual world features for immersive training experiences.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **GPS/GPX Route Import** | Convert real-world GPS routes to 3D worlds. Mercator projection + heightmap terrain. | Medium |
| **Dynamic Weather & Time-of-Day** | Rain, fog, snow particles. Dawn-to-night sky transitions. | Medium |
| **NPC Cyclists (Virtual Peloton)** | Ambient AI riders for group ride feel. Variable difficulty. | Medium |
| **Segment Leaderboards** | Route/segment-based rankings. Personal bests. Monthly challenges. | Medium |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Famous Pro Cycling Routes** | L'Alpe d'Huez, Mont Ventoux, Passo Gavia recreations with historical data. | Low-Medium |
| **Landmarks & Points of Interest** | Discoverable landmarks with info overlays. Achievement badges. | Low-Medium |
| **Difficulty Modifiers** | Flatten/steepen gradients. Gravity/drag multipliers. Accessibility options. | Low |
| **Adaptive Route Scaling** | Auto-scale difficulty to user's FTP for personalized challenge. | Low |
| **Virtual Drafting Mechanics** | 20-30% drag reduction behind NPCs. Strategic positioning. | Low-Medium |
| **Route Recommendation Engine** | Match routes to training goals, available time, fitness level. | Medium |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Procedural World Generation** | Infinite Perlin noise-based terrain. Biome selection. Seed-based reproducibility. | High |
| **World Creator Tools** | In-app level editor. Waypoint placement, height brushing, prop placement. | High |
| **Environmental Immersion Effects** | Audio cues tied to effort. Screen vignette on hard efforts. Haptic feedback. | Medium-High |
| **Coaching Overlay** | Real-time voice/text coaching. Adaptive pacing suggestions. | Medium |
| **Achievements & Collectibles** | In-world badge system. 50+ achievements. Collectible pickups. | Low-Medium |

---

## 4. Social & Multiplayer

Community features designed for self-hosted, offline-first architecture.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **LAN-Based Group Rides** | mDNS discovery. UDP metric sync. Real-time avatar synchronization. | Medium |
| **Local Leaderboards** | Segment/route rankings. SQLite storage. CSV/JSON export for sharing. | Medium |
| **Community Workout Repository** | 50-100 pre-curated workouts. Discover/filter by difficulty, focus. GitHub sync. | Medium |
| **Training Challenges** | User-created challenges ("50km this week"). TOML/JSON sharing. | Medium |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Virtual Race Events** | Scheduled races with entry lists, start sync, results ranking. | Medium |
| **Activity Feed** | Ride summaries shared via JSON files. mDNS feed discovery. | Low-Medium |
| **Workout Ratings & Reviews** | 1-5 star ratings, comments. Filter workout library by rating. | Medium |
| **Club Management** | Create/manage clubs. Member roster. Aggregate statistics. | Low-Medium |
| **Achievement Badges** | "First 100km", "FTP breakthrough", "Consistency" badges. Offline calculation. | Low-Medium |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Rider Profiles** | Name, avatar, bio, stats summary. Embedded in shared activity files. | Low-Medium |
| **LAN Chat** | Group ride messaging. Post-ride chat history. Simple rider-to-rider messages. | Low-Medium |
| **Ride Comparison Tool** | Head-to-head overlay charts. Compare vs personal best or friends. | Medium |
| **Async Group Workouts** | Complete workouts independently, aggregate leaderboard asynchronously. | Medium |

---

## 5. Hardware Integration

Expanded sensor and device support.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **ANT+ Protocol Support** | USB dongle support. Legacy trainer/sensor compatibility. | Medium-High |
| **Cycling Dynamics (L/R Balance)** | Extended Cycling Power Service parsing. Pedaling efficiency metrics. | Low-Medium |
| **Smart Trainer Incline Mode** | FTMS slope/grade simulation. Realistic gradient resistance. | Low |
| **Audio Cues / TTS** | Voice alerts for intervals, zones. Text-to-speech integration. | Low |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Smart Home Fan Control** | MQTT integration. Auto-adjust fan speed based on power/HR zones. | Medium |
| **External Display Support** | WebSocket streaming to tablet/TV. Real-time metrics dashboard. | Medium |
| **Weather Integration** | OpenWeatherMap API. Local conditions display during rides. | Low |
| **Stream Deck / USB Buttons** | HID device support. One-click lap markers, skip interval buttons. | Low-Medium |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Fitness Watch Integration** | Garmin/Apple Watch sync. HealthKit/Google Fit integration. | Medium |
| **Muscle Oxygen Monitoring** | WHOOP/Garmin muscle O2 sensors. SmO2 metrics. | Medium |
| **Video Course Sync** | MKV/MP4 playback synced to ride. Scenic route videos. | Medium-High |
| **Pedal Sensor Integration** | Shimano/Assioma pedal metrics. Force vector analysis. | Medium |
| **Cadence Sensor Fusion** | Dual sensor support. Kalman filter for accuracy. | Low |
| **Motion Tracking / IMU** | Rocker plate detection. Vibration analysis. | High |

---

## 6. UX & Accessibility

User experience improvements and accessibility compliance.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Keyboard Navigation** | Full Tab/Shift+Tab navigation. Prominent focus indicators. Shortcut guide. | Medium |
| **High Contrast / Colorblind Modes** | Protanopia, deuteranopia, tritanopia palettes. WCAG AAA contrast ratios. | Medium |
| **Onboarding Tutorial** | Step-by-step wizard. Interactive walkthroughs. Glossary tooltips. | Medium |
| **Imperial/Metric Unit Toggle** | User preference for km/h vs mph, km vs miles. System-wide conversion. | Low |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Dark/Light Theme Auto-Detection** | System theme detection. Scheduled switching. Custom theme builder. | Medium |
| **Customizable UI Layout** | Drag-and-drop metric widgets. Layout persistence. Preset profiles. | Medium-High |
| **Audio Feedback System** | Beeps for intervals, zone changes. Volume control. Custom audio profiles. | Medium |
| **Large Display / TV Mode** | 65"+ optimized. Dynamic font scaling. Spectator mode for groups. | Medium |
| **"Flow Mode"** | Minimal distractions. Single large metric. Full-screen 3D world. | Medium |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Multi-Language (i18n)** | fluent-rs localization. Spanish, French, German, Italian, Japanese. | Medium |
| **Screen Reader Support** | ARIA labels. Text-to-speech metrics. NVDA/VoiceOver compatible. | High |
| **Voice Control** | Hands-free commands. "Start Ride", "Pause", "Skip Interval". | High |
| **Mobile Companion App** | React Native/Flutter. Live metrics, remote control, ride history. | High |
| **Touch/Gesture Support** | Swipe navigation. Pinch-to-zoom. 44x44px touch targets. | Medium |
| **WCAG 2.1 AA Compliance Audit** | Automated testing. Quarterly reviews. Accessibility documentation. | High |

---

## 7. Platform & Technical

Infrastructure, deployment, and technical capabilities.

### High Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **REST API Server** | actix-web/axum. OAuth 2.0. WebSocket real-time streaming. | Medium-High |
| **Headless/CLI Mode** | Daemon without GUI. Scripting support. Raspberry Pi/server deployment. | Medium |
| **Fitness Platform Auto-Sync** | Strava, TrainingPeaks, Garmin Connect OAuth integration. FIT upload. | Medium |
| **Docker Deployment** | Multi-stage image (<300MB). docker-compose templates. Kubernetes manifests. | Medium |

### Medium Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Cloud Sync & Backup** | E2E encrypted sync. Nextcloud/S3 support. Multi-device ride history. | High |
| **Raspberry Pi Support** | ARM64 builds. Software rendering fallback. Pre-built Pi OS images. | Medium |
| **Advanced Analytics Engine** | 20-min power curves. Fitness trends. Workout effectiveness scores. | Medium |
| **Training Plan System** | Multi-week periodization. Progressive overload. Compliance tracking. | Medium-High |

### Lower Priority

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Plugin/Extension Architecture** | Dynamic plugin loading. WASM sandbox. Community extensions. | High |
| **Apple TV / Smart TV App** | tvOS app via REST API. Large display optimization. Remote control. | High |
| **AI Workout Recommendations** | TensorFlow Lite/ONNX. On-device inference. Personalized suggestions. | High |
| **Smart Home IoT Integration** | MQTT publisher. Home Assistant discovery. HomeKit support. | Medium-High |

---

## 8. Competitive Feature Gaps

Features from Zwift, TrainerRoad, Wahoo SYSTM, Rouvy, and Fulgaz that RustRide doesn't have.

### Highest Value Gaps

| Feature | Source | Description |
|---------|--------|-------------|
| **Adaptive AI Training Plans** | TrainerRoad | ML-driven plan adjustments from 250M+ activities |
| **Real-World Route Video** | Rouvy/Fulgaz | 35,000+ km of actual cycling route footage |
| **Real-Time Group Rides** | Zwift | 24/7 synchronized multiplayer cycling |
| **Racing Events & Leaderboards** | Zwift/Rouvy | Competitive events with live rankings |
| **Gradient-Responsive Resistance** | Rouvy | Auto-adjust trainer resistance from terrain |

### Medium Value Gaps

| Feature | Source | Description |
|---------|--------|-------------|
| **Achievement Badges & XP** | Zwift | 200+ badges, experience points, level progression |
| **Multi-Discipline Plans** | TrainerRoad | Road, gravel, triathlon, MTB-specific workouts |
| **4D Power Profiling** | Wahoo SYSTM | Beyond FTP: sprint, sustained, threshold, aerobic |
| **AR Route Overlays** | Rouvy | 3D elements over real video footage |
| **In-App Messaging** | Zwift | Real-time chat during rides |

### Lower Value Gaps

| Feature | Source | Description |
|---------|--------|-------------|
| **Outdoor Workout Sync** | Wahoo SYSTM | Execute indoor workouts outdoors on bike computer |
| **Multiple Themed Worlds** | Zwift | Urban, coastal, volcanic, fantasy environments |
| **Realistic Turn Physics** | Rouvy | Corner braking and deceleration mechanics |
| **Career Levels (80+)** | Rouvy | Long-term progression with partner discounts |

---

## Implementation Phases

### Phase 1: Foundation (Months 1-3)
- REST API Server
- Headless/CLI Mode
- ANT+ Protocol Support
- Keyboard Navigation & Colorblind Modes
- Local Leaderboards
- Power Duration Curve

### Phase 2: Core Analytics (Months 4-6)
- AI FTP Detection
- CP/W' Model
- Adaptive Workout Recommendations
- Periodization Builder
- Fitness Platform Auto-Sync (Strava)
- Community Workout Repository

### Phase 3: Social & Content (Months 7-9)
- LAN Group Rides
- GPX Route Import
- Dynamic Weather/Time-of-Day
- Training Challenges
- Achievement Badges
- NPC Cyclists

### Phase 4: Advanced Features (Months 10-12)
- Real-Time Fatigue Detection
- Procedural World Generation
- Cloud Sync & Backup
- Mobile Companion App
- Smart Home Integration
- Famous Pro Routes

### Phase 5: Polish & Scale (Months 13+)
- Plugin Architecture
- Multi-Language Support
- Apple TV / Smart TV
- Voice Control
- WCAG Compliance Audit
- World Creator Tools

---

## Research Sources

This feature list was compiled from:
- Competitive analysis of Zwift, TrainerRoad, Wahoo SYSTM, Rouvy, Fulgaz
- Sports science literature on training metrics
- Self-hosted fitness platform patterns (wger, Endurain)
- Rust ML ecosystem research (polars, linfa, burn)
- Accessibility standards (WCAG 2.1)
- Indoor cycling community feedback

---

*Generated: 2025-12-25*
