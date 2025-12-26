//! Scene graph for the 3D world
//!
//! T053: Add weather/time settings to scene configuration
//! T144: Add contextual audio system for immersion

use super::weather::{TimeOfDay, WeatherState, WeatherType};
use glam::{Quat, Vec3};

/// Transform component for scene objects
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Position in world space
    pub position: Vec3,
    /// Rotation as a quaternion
    pub rotation: Quat,
    /// Scale factor
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Create a transform at the given position
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Create a transform with position and rotation
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Default::default()
        }
    }
}

/// Handle to a loaded model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelHandle(pub u32);

/// A scenery object in the world
#[derive(Debug, Clone)]
pub struct SceneryObject {
    /// Model to render
    pub model: ModelHandle,
    /// Transform in world space
    pub transform: Transform,
    /// Distance at which to use lower LOD
    pub lod_distance: f32,
}

/// Lighting configuration
#[derive(Debug, Clone)]
pub struct Lighting {
    /// Direction of the sun/main light
    pub sun_direction: Vec3,
    /// Ambient light color and intensity
    pub ambient_color: Vec3,
    /// Sun light color and intensity
    pub sun_color: Vec3,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(0.5, 1.0, 0.3).normalize(),
            ambient_color: Vec3::new(0.3, 0.3, 0.35),
            sun_color: Vec3::new(1.0, 0.95, 0.9),
        }
    }
}

/// Sky configuration
#[derive(Debug, Clone)]
pub struct Sky {
    /// Top color of sky gradient
    pub top_color: Vec3,
    /// Horizon color of sky gradient
    pub horizon_color: Vec3,
    /// Sun color for sky rendering
    pub sun_color: Vec3,
    /// Fog color (blends with horizon at distance)
    pub fog_color: Vec3,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            top_color: Vec3::new(0.4, 0.6, 1.0),
            horizon_color: Vec3::new(0.8, 0.85, 0.95),
            sun_color: Vec3::new(1.0, 0.95, 0.8),
            fog_color: Vec3::new(0.7, 0.75, 0.8),
        }
    }
}

/// Fog configuration (T053)
#[derive(Debug, Clone)]
pub struct FogSettings {
    /// Fog color
    pub color: Vec3,
    /// Start distance for fog
    pub start_distance: f32,
    /// End distance where fog is fully opaque
    pub end_distance: f32,
    /// Fog density (0.0-1.0)
    pub density: f32,
}

impl Default for FogSettings {
    fn default() -> Self {
        Self {
            color: Vec3::new(0.7, 0.75, 0.8),
            start_distance: 100.0,
            end_distance: 1000.0,
            density: 0.0,
        }
    }
}

impl FogSettings {
    /// Create fog settings for a weather type (T053)
    pub fn for_weather(weather: WeatherType) -> Self {
        match weather {
            WeatherType::Clear => Self {
                color: Vec3::new(0.8, 0.85, 0.95),
                start_distance: 500.0,
                end_distance: 5000.0,
                density: 0.1,
            },
            WeatherType::Cloudy => Self {
                color: Vec3::new(0.7, 0.75, 0.8),
                start_distance: 300.0,
                end_distance: 3000.0,
                density: 0.2,
            },
            WeatherType::Rain => Self {
                color: Vec3::new(0.5, 0.55, 0.6),
                start_distance: 100.0,
                end_distance: 1500.0,
                density: 0.4,
            },
            WeatherType::HeavyRain => Self {
                color: Vec3::new(0.4, 0.45, 0.5),
                start_distance: 50.0,
                end_distance: 800.0,
                density: 0.6,
            },
            WeatherType::Fog => Self {
                color: Vec3::new(0.8, 0.82, 0.85),
                start_distance: 10.0,
                end_distance: 200.0,
                density: 0.9,
            },
            WeatherType::Snow => Self {
                color: Vec3::new(0.85, 0.88, 0.92),
                start_distance: 80.0,
                end_distance: 1000.0,
                density: 0.5,
            },
        }
    }
}

/// Weather settings for the scene (T053)
#[derive(Debug, Clone)]
pub struct WeatherSettings {
    /// Current weather type
    pub weather_type: WeatherType,
    /// Current time of day
    pub time_of_day: TimeOfDay,
    /// Time in hours (0.0-24.0)
    pub time_hours: f32,
    /// Fog settings derived from weather
    pub fog: FogSettings,
    /// Rain/snow intensity (0.0-1.0)
    pub precipitation_intensity: f32,
    /// Wind speed in m/s
    pub wind_speed: f32,
    /// Wind direction in radians
    pub wind_direction: f32,
}

impl Default for WeatherSettings {
    fn default() -> Self {
        Self {
            weather_type: WeatherType::Clear,
            time_of_day: TimeOfDay::Day,
            time_hours: 12.0,
            fog: FogSettings::default(),
            precipitation_intensity: 0.0,
            wind_speed: 0.0,
            wind_direction: 0.0,
        }
    }
}

impl WeatherSettings {
    /// Update settings from a weather state (T053)
    pub fn from_weather_state(state: &WeatherState) -> Self {
        let fog = FogSettings::for_weather(state.weather);
        Self {
            weather_type: state.weather,
            time_of_day: state.time_of_day,
            time_hours: state.time_hours,
            fog,
            precipitation_intensity: state.current_particle_density(),
            wind_speed: state.wind_speed_kmh / 3.6, // Convert km/h to m/s
            wind_direction: state.wind_direction_degrees.to_radians(),
        }
    }
}

/// The complete scene to render
#[derive(Debug, Default)]
pub struct Scene {
    /// Scenery objects (trees, buildings, etc.)
    pub scenery: Vec<SceneryObject>,
    /// Lighting configuration
    pub lighting: Lighting,
    /// Sky configuration
    pub sky: Sky,
    /// Weather settings (T053)
    pub weather: WeatherSettings,
}

impl Scene {
    /// Create a new empty scene
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a scenery object to the scene
    pub fn add_scenery(&mut self, object: SceneryObject) {
        self.scenery.push(object);
    }

    /// Clear all scenery objects
    pub fn clear_scenery(&mut self) {
        self.scenery.clear();
    }

    /// Update scene from weather state (T053)
    pub fn apply_weather(&mut self, state: &WeatherState) {
        use super::weather::skybox::{ambient_light, sun_position, SkyColors};

        // Update weather settings
        self.weather = WeatherSettings::from_weather_state(state);

        // Update sky colors from weather
        let sky_colors = SkyColors::for_conditions(state.time_hours, state.weather);
        self.sky.top_color = sky_colors.zenith;
        self.sky.horizon_color = sky_colors.horizon;
        self.sky.sun_color = sky_colors.sun;
        self.sky.fog_color = sky_colors.fog;

        // Update lighting from weather and time
        let sun_dir = sun_position(state.time_hours);
        self.lighting.sun_direction = sun_dir;
        self.lighting.sun_color = sky_colors.sun;

        let (ambient_color, ambient_intensity) = ambient_light(state.time_hours, state.weather);
        self.lighting.ambient_color = ambient_color * ambient_intensity;
    }
}

// ========== T144: Contextual Audio System ==========

/// Types of environmental audio sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioType {
    /// Ambient wind sound
    Wind,
    /// Rain on surfaces
    Rain,
    /// Heavy storm
    Storm,
    /// Snow/quiet precipitation
    Snow,
    /// Birds singing (day)
    Birds,
    /// Crickets (night)
    Crickets,
    /// Water/stream nearby
    Water,
    /// Tire on road
    TireRoll,
    /// Drivetrain/chain
    Drivetrain,
    /// Breathing (effort-based)
    Breathing,
    /// Heartbeat (high effort)
    Heartbeat,
}

impl AudioType {
    /// Get the base filename for this audio type
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Wind => "wind_loop",
            Self::Rain => "rain_loop",
            Self::Storm => "storm_loop",
            Self::Snow => "snow_ambient",
            Self::Birds => "birds_day",
            Self::Crickets => "crickets_night",
            Self::Water => "stream_water",
            Self::TireRoll => "tire_road",
            Self::Drivetrain => "drivetrain_hum",
            Self::Breathing => "breathing_heavy",
            Self::Heartbeat => "heartbeat",
        }
    }

    /// Whether this audio loops continuously
    pub fn is_looping(&self) -> bool {
        match self {
            Self::Birds | Self::Crickets => false, // One-shots with random timing
            _ => true,
        }
    }
}

/// An audio source in the scene
#[derive(Debug, Clone)]
pub struct AudioSource {
    /// Type of audio
    pub audio_type: AudioType,
    /// Volume (0.0-1.0)
    pub volume: f32,
    /// Target volume for smooth transitions
    pub target_volume: f32,
    /// Pitch multiplier (1.0 = normal)
    pub pitch: f32,
    /// Whether this source is currently playing
    pub playing: bool,
    /// Position in 3D space (None for ambient/non-positional)
    pub position: Option<Vec3>,
    /// Priority for audio mixing
    pub priority: u8,
}

impl AudioSource {
    /// Create a new audio source
    pub fn new(audio_type: AudioType) -> Self {
        Self {
            audio_type,
            volume: 0.0,
            target_volume: 0.0,
            pitch: 1.0,
            playing: false,
            position: None,
            priority: 50,
        }
    }

    /// Create an ambient (non-positional) source
    pub fn ambient(audio_type: AudioType, priority: u8) -> Self {
        Self {
            audio_type,
            volume: 0.0,
            target_volume: 0.0,
            pitch: 1.0,
            playing: false,
            position: None,
            priority,
        }
    }

    /// Update volume smoothly toward target
    pub fn update(&mut self, delta_time: f32) {
        const FADE_SPEED: f32 = 2.0; // Volume change per second

        if (self.volume - self.target_volume).abs() < 0.01 {
            self.volume = self.target_volume;
        } else if self.volume < self.target_volume {
            self.volume = (self.volume + FADE_SPEED * delta_time).min(self.target_volume);
        } else {
            self.volume = (self.volume - FADE_SPEED * delta_time).max(self.target_volume);
        }

        self.playing = self.volume > 0.01;
    }

    /// Set target volume with smooth transition
    pub fn fade_to(&mut self, volume: f32) {
        self.target_volume = volume.clamp(0.0, 1.0);
    }

    /// Set volume immediately (no fade)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.target_volume = self.volume;
        self.playing = self.volume > 0.01;
    }
}

/// Audio context for the scene
#[derive(Debug, Clone)]
pub struct AudioContext {
    /// Master volume (0.0-1.0)
    pub master_volume: f32,
    /// Environmental audio sources
    pub environment: Vec<AudioSource>,
    /// Cyclist audio sources (effort-based)
    pub cyclist: Vec<AudioSource>,
    /// Whether audio is enabled
    pub enabled: bool,
    /// Current biome (affects ambient sounds)
    pub current_biome: Option<String>,
    /// Current effort level (0.0-2.0)
    pub effort_level: f32,
    /// Current speed in m/s
    pub speed_mps: f32,
}

impl Default for AudioContext {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioContext {
    /// Create a new audio context with default sources
    pub fn new() -> Self {
        let environment = vec![
            AudioSource::ambient(AudioType::Wind, 80),
            AudioSource::ambient(AudioType::Rain, 90),
            AudioSource::ambient(AudioType::Storm, 95),
            AudioSource::ambient(AudioType::Snow, 70),
            AudioSource::ambient(AudioType::Birds, 30),
            AudioSource::ambient(AudioType::Crickets, 30),
            AudioSource::ambient(AudioType::Water, 40),
        ];

        let cyclist = vec![
            AudioSource::ambient(AudioType::TireRoll, 60),
            AudioSource::ambient(AudioType::Drivetrain, 50),
            AudioSource::ambient(AudioType::Breathing, 70),
            AudioSource::ambient(AudioType::Heartbeat, 85),
        ];

        Self {
            master_volume: 0.8,
            environment,
            cyclist,
            enabled: true,
            current_biome: None,
            effort_level: 0.0,
            speed_mps: 0.0,
        }
    }

    /// Update audio based on weather state (T145)
    pub fn update_weather(&mut self, weather: &WeatherState) {
        if !self.enabled {
            return;
        }

        // Wind audio based on wind speed
        if let Some(wind) = self.find_env_mut(AudioType::Wind) {
            let wind_volume = (weather.wind_speed_kmh / 50.0).clamp(0.1, 1.0);
            wind.fade_to(wind_volume);
            wind.pitch = 0.8 + (weather.wind_speed_kmh / 100.0).clamp(0.0, 0.4);
        }

        // Rain audio
        if let Some(rain) = self.find_env_mut(AudioType::Rain) {
            match weather.weather {
                WeatherType::Rain => {
                    rain.fade_to(0.7);
                }
                WeatherType::HeavyRain => {
                    rain.fade_to(1.0);
                }
                _ => {
                    rain.fade_to(0.0);
                }
            }
        }

        // Storm audio
        if let Some(storm) = self.find_env_mut(AudioType::Storm) {
            if weather.weather == WeatherType::HeavyRain {
                storm.fade_to(0.8);
            } else {
                storm.fade_to(0.0);
            }
        }

        // Snow audio (quiet ambient)
        if let Some(snow) = self.find_env_mut(AudioType::Snow) {
            if weather.weather == WeatherType::Snow {
                snow.fade_to(0.5);
            } else {
                snow.fade_to(0.0);
            }
        }

        // Day/night ambient sounds
        let is_daytime = matches!(weather.time_of_day, TimeOfDay::Day | TimeOfDay::Dawn);
        let is_clear = matches!(weather.weather, WeatherType::Clear | WeatherType::Cloudy);

        if let Some(birds) = self.find_env_mut(AudioType::Birds) {
            if is_daytime && is_clear {
                birds.fade_to(0.4);
            } else {
                birds.fade_to(0.0);
            }
        }

        if let Some(crickets) = self.find_env_mut(AudioType::Crickets) {
            if !is_daytime && is_clear {
                crickets.fade_to(0.3);
            } else {
                crickets.fade_to(0.0);
            }
        }
    }

    /// Update cyclist audio based on effort and speed
    pub fn update_cyclist(&mut self, effort_level: f32, speed_mps: f32) {
        if !self.enabled {
            return;
        }

        self.effort_level = effort_level;
        self.speed_mps = speed_mps;

        // Tire roll - based on speed
        if let Some(tire) = self.find_cyclist_mut(AudioType::TireRoll) {
            if speed_mps > 0.5 {
                let volume = (speed_mps / 15.0).clamp(0.2, 1.0);
                tire.fade_to(volume);
                tire.pitch = 0.8 + (speed_mps / 20.0).clamp(0.0, 0.4);
            } else {
                tire.fade_to(0.0);
            }
        }

        // Drivetrain - based on speed
        if let Some(drivetrain) = self.find_cyclist_mut(AudioType::Drivetrain) {
            if speed_mps > 1.0 {
                let volume = (speed_mps / 20.0).clamp(0.1, 0.6);
                drivetrain.fade_to(volume);
                drivetrain.pitch = 0.9 + (speed_mps / 25.0).clamp(0.0, 0.3);
            } else {
                drivetrain.fade_to(0.0);
            }
        }

        // Breathing - based on effort
        if let Some(breathing) = self.find_cyclist_mut(AudioType::Breathing) {
            if effort_level > 0.75 {
                let volume = ((effort_level - 0.75) / 0.45).clamp(0.0, 1.0);
                breathing.fade_to(volume * 0.7);
                breathing.pitch = 0.9 + (effort_level - 0.75).clamp(0.0, 0.3);
            } else {
                breathing.fade_to(0.0);
            }
        }

        // Heartbeat - at very high effort
        if let Some(heartbeat) = self.find_cyclist_mut(AudioType::Heartbeat) {
            if effort_level > 0.95 {
                let volume = ((effort_level - 0.95) / 0.25).clamp(0.0, 1.0);
                heartbeat.fade_to(volume * 0.5);
                // Faster heartbeat at higher effort
                heartbeat.pitch = 0.8 + effort_level.clamp(0.0, 0.4);
            } else {
                heartbeat.fade_to(0.0);
            }
        }
    }

    /// Update all audio sources
    pub fn update(&mut self, delta_time: f32) {
        for source in &mut self.environment {
            source.update(delta_time);
        }
        for source in &mut self.cyclist {
            source.update(delta_time);
        }
    }

    /// Get all currently playing sources
    pub fn playing_sources(&self) -> Vec<&AudioSource> {
        let mut sources: Vec<&AudioSource> = self
            .environment
            .iter()
            .chain(self.cyclist.iter())
            .filter(|s| s.playing)
            .collect();

        // Sort by priority (higher = more important)
        sources.sort_by(|a, b| b.priority.cmp(&a.priority));
        sources
    }

    /// Find environment source by type
    fn find_env_mut(&mut self, audio_type: AudioType) -> Option<&mut AudioSource> {
        self.environment
            .iter_mut()
            .find(|s| s.audio_type == audio_type)
    }

    /// Find cyclist source by type
    fn find_cyclist_mut(&mut self, audio_type: AudioType) -> Option<&mut AudioSource> {
        self.cyclist.iter_mut().find(|s| s.audio_type == audio_type)
    }

    /// Mute all audio
    pub fn mute(&mut self) {
        for source in &mut self.environment {
            source.fade_to(0.0);
        }
        for source in &mut self.cyclist {
            source.fade_to(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_source_fade() {
        let mut source = AudioSource::new(AudioType::Wind);
        source.fade_to(0.8);

        // Should be at 0 initially
        assert_eq!(source.volume, 0.0);
        assert!(!source.playing);

        // Update should move toward target (small delta to see incremental progress)
        source.update(0.1); // FADE_SPEED=2.0, so volume += 0.2 per update
        assert!(source.volume > 0.0, "Volume should increase after update");
        assert!(source.volume < 0.8, "Volume should not overshoot target");

        // Multiple updates should reach target
        for _ in 0..10 {
            source.update(0.1);
        }
        assert!(
            (source.volume - 0.8).abs() < 0.01,
            "Volume should reach target"
        );
        assert!(source.playing, "Source should be playing at target volume");
    }

    #[test]
    fn test_audio_context_weather() {
        let mut ctx = AudioContext::new();

        let weather = WeatherState {
            weather: WeatherType::Rain,
            time_of_day: TimeOfDay::Day,
            time_hours: 12.0,
            wind_speed_kmh: 20.0,
            wind_direction_degrees: 0.0,
            visibility_meters: 5000.0,
            transition_progress: 1.0,
            previous_weather: None,
            realistic_time: false,
        };

        ctx.update_weather(&weather);

        // Rain should be fading in
        let rain = ctx
            .environment
            .iter()
            .find(|s| s.audio_type == AudioType::Rain)
            .unwrap();
        assert!(rain.target_volume > 0.0);
    }

    #[test]
    fn test_audio_context_cyclist() {
        let mut ctx = AudioContext::new();

        // High effort, moving
        ctx.update_cyclist(1.2, 10.0);

        // Tire roll should be active
        let tire = ctx
            .cyclist
            .iter()
            .find(|s| s.audio_type == AudioType::TireRoll)
            .unwrap();
        assert!(tire.target_volume > 0.0);

        // Breathing should be active at high effort
        let breathing = ctx
            .cyclist
            .iter()
            .find(|s| s.audio_type == AudioType::Breathing)
            .unwrap();
        assert!(breathing.target_volume > 0.0);

        // Heartbeat should be active above 0.95 effort
        let heartbeat = ctx
            .cyclist
            .iter()
            .find(|s| s.audio_type == AudioType::Heartbeat)
            .unwrap();
        assert!(heartbeat.target_volume > 0.0);
    }

    #[test]
    fn test_audio_type_properties() {
        assert!(AudioType::Wind.is_looping());
        assert!(!AudioType::Birds.is_looping());
        assert_eq!(AudioType::Rain.filename(), "rain_loop");
    }
}
