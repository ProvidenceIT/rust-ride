//! Main gradient controller coordinating route, smoothing, and resistance.

use uuid::Uuid;

use crate::gradient::error::GradientError;
use crate::gradient::resistance::ResistanceController;
use crate::gradient::route::GpxRoute;
use crate::gradient::settings::GradientSettings;
use crate::gradient::smoothing::GradientSmoother;
use crate::gradient::types::GradientResult;

/// Controller for managing gradient-responsive resistance.
pub struct GradientController {
    /// Currently loaded route
    route: Option<GpxRoute>,
    /// Resistance controller for FTMS commands
    resistance: ResistanceController,
    /// Gradient smoother
    smoother: GradientSmoother,
    /// Current position along the route (meters)
    current_distance_m: f64,
    /// Update rate in Hz
    update_rate_hz: u8,
}

impl GradientController {
    /// Create a new gradient controller.
    ///
    /// # Arguments
    /// * `update_rate_hz` - How often the controller will be updated (typically 1-10 Hz)
    pub fn new(update_rate_hz: u8) -> Self {
        Self {
            route: None,
            resistance: ResistanceController::new(),
            smoother: GradientSmoother::from_seconds(3, update_rate_hz),
            current_distance_m: 0.0,
            update_rate_hz,
        }
    }

    /// Load a route for gradient simulation.
    pub fn load_route(&mut self, route: GpxRoute) -> Result<(), GradientError> {
        if !route.has_elevation_data() {
            return Err(GradientError::NoElevationData);
        }

        self.route = Some(route);
        self.reset_position();
        Ok(())
    }

    /// Load a route by ID (placeholder for database integration).
    pub fn load_route_by_id(&mut self, _route_id: Uuid) -> Result<(), GradientError> {
        // TODO: Load from database
        Err(GradientError::RouteNotFound(_route_id))
    }

    /// Unload the current route.
    pub fn unload_route(&mut self) {
        self.route = None;
        self.reset_position();
    }

    /// Reset position to start of route.
    pub fn reset_position(&mut self) {
        self.current_distance_m = 0.0;
        self.smoother.reset();
    }

    /// Update the gradient controller with a new position.
    ///
    /// # Arguments
    /// * `distance_m` - Current distance along the route in meters
    /// * `settings` - User's gradient settings
    ///
    /// # Returns
    /// Gradient result and optional FTMS command bytes
    pub fn update(
        &mut self,
        distance_m: f64,
        settings: &GradientSettings,
    ) -> Result<(GradientResult, Option<Vec<u8>>), GradientError> {
        let route = self.route.as_ref().ok_or(GradientError::NoRouteLoaded)?;

        // Validate position
        if distance_m > route.total_distance_m {
            return Err(GradientError::PositionOutOfBounds {
                position: distance_m,
                max: route.total_distance_m,
            });
        }

        self.current_distance_m = distance_m;

        // Get raw gradient from route
        let raw_gradient = route.get_gradient_at(distance_m);

        // Apply capping
        let capped_gradient = settings.clamp_gradient(raw_gradient);

        // Apply difficulty scaling
        let effective_gradient = settings.apply_difficulty(capped_gradient);

        // Apply smoothing
        let smoothed_gradient = self.smoother.add(effective_gradient);

        // Generate FTMS command if gradient changed significantly
        let command = self.resistance.update(smoothed_gradient, settings);

        let result = GradientResult::from_raw(
            raw_gradient,
            capped_gradient,
            effective_gradient,
            smoothed_gradient,
        );

        Ok((result, command))
    }

    /// Update smoothing parameters based on settings.
    pub fn update_smoothing(&mut self, settings: &GradientSettings) {
        self.smoother = GradientSmoother::from_seconds(
            settings.smoothing_secs,
            self.update_rate_hz,
        );
    }

    /// Check if a route is loaded.
    pub fn has_route(&self) -> bool {
        self.route.is_some()
    }

    /// Check if the loaded route has elevation data.
    pub fn has_elevation_data(&self) -> bool {
        self.route.as_ref().map(|r| r.has_elevation_data()).unwrap_or(false)
    }

    /// Get the total route distance.
    pub fn total_distance(&self) -> Option<f64> {
        self.route.as_ref().map(|r| r.total_distance_m)
    }

    /// Get the total elevation gain.
    pub fn total_elevation_gain(&self) -> Option<f64> {
        self.route.as_ref().map(|r| r.total_elevation_m)
    }

    /// Get the current position along the route.
    pub fn current_distance(&self) -> f64 {
        self.current_distance_m
    }

    /// Get progress as a percentage (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        self.route
            .as_ref()
            .map(|r| r.get_progress(self.current_distance_m))
            .unwrap_or(0.0)
    }

    /// Get the current elevation at the rider's position.
    pub fn current_elevation(&self) -> Option<f32> {
        self.route
            .as_ref()
            .and_then(|r| r.get_elevation_at(self.current_distance_m))
    }

    /// Get the loaded route reference.
    pub fn route(&self) -> Option<&GpxRoute> {
        self.route.as_ref()
    }

    /// Get a command to reset trainer to flat.
    pub fn reset_resistance(&mut self) -> Vec<u8> {
        self.resistance.reset()
    }
}

impl Default for GradientController {
    fn default() -> Self {
        Self::new(1) // 1 Hz default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient::types::RoutePoint;

    fn create_test_route() -> GpxRoute {
        let points = vec![
            RoutePoint::new(0.0, 100.0),
            RoutePoint::new(100.0, 110.0),
            RoutePoint::new(200.0, 120.0),
            RoutePoint::new(300.0, 115.0),
            RoutePoint::new(400.0, 110.0),
        ];
        GpxRoute::from_points("Test", points).unwrap()
    }

    #[test]
    fn test_controller_creation() {
        let controller = GradientController::new(10);
        assert!(!controller.has_route());
        assert_eq!(controller.current_distance(), 0.0);
    }

    #[test]
    fn test_load_route() {
        let mut controller = GradientController::new(10);
        let route = create_test_route();

        controller.load_route(route).unwrap();
        assert!(controller.has_route());
        assert!(controller.has_elevation_data());
        assert_eq!(controller.total_distance(), Some(400.0));
    }

    #[test]
    fn test_update() {
        let mut controller = GradientController::new(10);
        let route = create_test_route();
        let settings = GradientSettings::default();

        controller.load_route(route).unwrap();

        let (result, command) = controller.update(50.0, &settings).unwrap();

        // Should have a gradient (climbing section)
        assert!(result.raw_gradient > 0.0);
        // First update should generate a command
        assert!(command.is_some());
    }

    #[test]
    fn test_no_route_error() {
        let mut controller = GradientController::new(10);
        let settings = GradientSettings::default();

        let result = controller.update(50.0, &settings);
        assert!(matches!(result, Err(GradientError::NoRouteLoaded)));
    }

    #[test]
    fn test_position_out_of_bounds() {
        let mut controller = GradientController::new(10);
        let route = create_test_route();
        let settings = GradientSettings::default();

        controller.load_route(route).unwrap();

        let result = controller.update(500.0, &settings);
        assert!(matches!(result, Err(GradientError::PositionOutOfBounds { .. })));
    }

    #[test]
    fn test_progress() {
        let mut controller = GradientController::new(10);
        let route = create_test_route();
        let settings = GradientSettings::default();

        controller.load_route(route).unwrap();
        controller.update(200.0, &settings).unwrap();

        assert!((controller.progress() - 0.5).abs() < 0.01);
    }
}
