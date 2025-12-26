//! World creator tools for route and environment editing.

pub mod serialization;
pub mod tools;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Creator mode state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatorMode {
    #[default]
    /// Not in creator mode
    Disabled,
    /// Viewing/navigating
    View,
    /// Editing route path
    EditRoute,
    /// Editing terrain
    EditTerrain,
    /// Placing objects
    PlaceObjects,
    /// Editing landmarks
    EditLandmarks,
}

/// Object types that can be placed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceableType {
    /// Tree
    Tree,
    /// Rock
    Rock,
    /// Building
    Building,
    /// Sign/banner
    Sign,
    /// Barrier/fence
    Barrier,
    /// Light post
    Light,
    /// Spectator group
    Spectators,
    /// Custom marker
    Marker,
}

/// A placed object in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedObject {
    /// Unique identifier
    pub id: Uuid,
    /// Object type
    pub object_type: PlaceableType,
    /// World position
    pub position: Vec3,
    /// Y-axis rotation (radians)
    pub rotation: f32,
    /// Uniform scale
    pub scale: f32,
    /// Optional variant index (for different models)
    pub variant: u8,
}

impl PlacedObject {
    /// Create new placed object
    pub fn new(object_type: PlaceableType, position: Vec3) -> Self {
        Self {
            id: Uuid::new_v4(),
            object_type,
            position,
            rotation: 0.0,
            scale: 1.0,
            variant: 0,
        }
    }

    /// Set rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

/// Route point for custom routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePoint {
    /// GPS latitude
    pub latitude: f64,
    /// GPS longitude
    pub longitude: f64,
    /// Elevation in meters
    pub elevation: f32,
    /// Width of road at this point (meters)
    pub road_width: f32,
    /// Surface type at this point
    pub surface: super::procedural::SurfaceType,
}

impl RoutePoint {
    /// Create route point
    pub fn new(latitude: f64, longitude: f64, elevation: f32) -> Self {
        Self {
            latitude,
            longitude,
            elevation,
            road_width: 6.0,
            surface: super::procedural::SurfaceType::Asphalt,
        }
    }
}

/// Custom route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRoute {
    /// Unique identifier
    pub id: Uuid,
    /// Route name
    pub name: String,
    /// Route points
    pub points: Vec<RoutePoint>,
    /// Placed objects
    pub objects: Vec<PlacedObject>,
    /// Whether route is closed loop
    pub is_loop: bool,
    /// Author user ID
    pub author_id: Option<Uuid>,
}

impl CustomRoute {
    /// Create new empty custom route
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            points: Vec::new(),
            objects: Vec::new(),
            is_loop: false,
            author_id: None,
        }
    }

    /// Add a point to the route
    pub fn add_point(&mut self, point: RoutePoint) {
        self.points.push(point);
    }

    /// Remove last point
    pub fn remove_last_point(&mut self) {
        self.points.pop();
    }

    /// Insert point at index
    pub fn insert_point(&mut self, index: usize, point: RoutePoint) {
        if index <= self.points.len() {
            self.points.insert(index, point);
        }
    }

    /// Remove point at index
    pub fn remove_point(&mut self, index: usize) {
        if index < self.points.len() {
            self.points.remove(index);
        }
    }

    /// Calculate total distance
    pub fn total_distance(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }

        let mut total = 0.0;
        for i in 1..self.points.len() {
            let p1 = &self.points[i - 1];
            let p2 = &self.points[i];
            total += crate::world::import::haversine_distance(
                p1.latitude,
                p1.longitude,
                p2.latitude,
                p2.longitude,
            );
        }

        // Add distance back to start if loop
        if self.is_loop && self.points.len() > 2 {
            let first = &self.points[0];
            let last = &self.points[self.points.len() - 1];
            total += crate::world::import::haversine_distance(
                first.latitude,
                first.longitude,
                last.latitude,
                last.longitude,
            );
        }

        total
    }

    /// Calculate total elevation gain
    pub fn total_elevation_gain(&self) -> f32 {
        if self.points.len() < 2 {
            return 0.0;
        }

        let mut gain = 0.0;
        for i in 1..self.points.len() {
            let delta = self.points[i].elevation - self.points[i - 1].elevation;
            if delta > 0.0 {
                gain += delta;
            }
        }

        gain
    }

    /// Add placed object
    pub fn add_object(&mut self, object: PlacedObject) {
        self.objects.push(object);
    }

    /// Remove object by ID
    pub fn remove_object(&mut self, id: Uuid) {
        self.objects.retain(|o| o.id != id);
    }

    /// Find object by ID
    pub fn find_object(&self, id: Uuid) -> Option<&PlacedObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    /// Find mutable object by ID
    pub fn find_object_mut(&mut self, id: Uuid) -> Option<&mut PlacedObject> {
        self.objects.iter_mut().find(|o| o.id == id)
    }
}

/// World creator state
pub struct WorldCreator {
    /// Current mode
    mode: CreatorMode,
    /// Route being edited
    current_route: Option<CustomRoute>,
    /// Currently selected object
    selected_object: Option<Uuid>,
    /// Currently selected point index
    selected_point: Option<usize>,
    /// Undo stack
    undo_stack: Vec<CreatorAction>,
    /// Redo stack
    redo_stack: Vec<CreatorAction>,
    /// Grid snap size (meters)
    grid_size: f32,
    /// Whether grid snap is enabled
    snap_to_grid: bool,
}

/// Undoable creator action
#[derive(Debug, Clone)]
pub enum CreatorAction {
    /// Added a route point
    AddPoint(RoutePoint),
    /// Removed a route point
    RemovePoint(usize, RoutePoint),
    /// Moved a route point
    MovePoint(usize, RoutePoint, RoutePoint), // index, old, new
    /// Added an object
    AddObject(PlacedObject),
    /// Removed an object
    RemoveObject(PlacedObject),
    /// Moved an object
    MoveObject(Uuid, Vec3, Vec3), // id, old_pos, new_pos
}

impl WorldCreator {
    /// Create world creator
    pub fn new() -> Self {
        Self {
            mode: CreatorMode::Disabled,
            current_route: None,
            selected_object: None,
            selected_point: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            grid_size: 1.0,
            snap_to_grid: false,
        }
    }

    /// Get current mode
    pub fn mode(&self) -> CreatorMode {
        self.mode
    }

    /// Set mode
    pub fn set_mode(&mut self, mode: CreatorMode) {
        self.mode = mode;
    }

    /// Get current route
    pub fn route(&self) -> Option<&CustomRoute> {
        self.current_route.as_ref()
    }

    /// Get mutable current route
    pub fn route_mut(&mut self) -> Option<&mut CustomRoute> {
        self.current_route.as_mut()
    }

    /// Start new route
    pub fn new_route(&mut self, name: String) {
        self.current_route = Some(CustomRoute::new(name));
        self.selected_object = None;
        self.selected_point = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Load existing route for editing
    pub fn load_route(&mut self, route: CustomRoute) {
        self.current_route = Some(route);
        self.selected_object = None;
        self.selected_point = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Add point to route
    pub fn add_point(&mut self, point: RoutePoint) {
        if let Some(route) = &mut self.current_route {
            let action = CreatorAction::AddPoint(point.clone());
            route.add_point(point);
            self.undo_stack.push(action);
            self.redo_stack.clear();
        }
    }

    /// Add object to route
    pub fn add_object(&mut self, object: PlacedObject) {
        if let Some(route) = &mut self.current_route {
            let action = CreatorAction::AddObject(object.clone());
            route.add_object(object);
            self.undo_stack.push(action);
            self.redo_stack.clear();
        }
    }

    /// Select object
    pub fn select_object(&mut self, id: Option<Uuid>) {
        self.selected_object = id;
        self.selected_point = None;
    }

    /// Select point
    pub fn select_point(&mut self, index: Option<usize>) {
        self.selected_point = index;
        self.selected_object = None;
    }

    /// Get selected object
    pub fn selected_object(&self) -> Option<Uuid> {
        self.selected_object
    }

    /// Get selected point
    pub fn selected_point(&self) -> Option<usize> {
        self.selected_point
    }

    /// Undo last action
    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            if let Some(route) = &mut self.current_route {
                match &action {
                    CreatorAction::AddPoint(_) => {
                        route.remove_last_point();
                    }
                    CreatorAction::RemovePoint(idx, point) => {
                        route.insert_point(*idx, point.clone());
                    }
                    CreatorAction::AddObject(_) => {
                        if let Some(obj) = route.objects.last() {
                            route.remove_object(obj.id);
                        }
                    }
                    CreatorAction::RemoveObject(obj) => {
                        route.add_object(obj.clone());
                    }
                    CreatorAction::MoveObject(id, old_pos, _) => {
                        if let Some(obj) = route.find_object_mut(*id) {
                            obj.position = *old_pos;
                        }
                    }
                    CreatorAction::MovePoint(idx, old, _) => {
                        if *idx < route.points.len() {
                            route.points[*idx] = old.clone();
                        }
                    }
                }
            }
            self.redo_stack.push(action);
        }
    }

    /// Redo last undone action
    pub fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            if let Some(route) = &mut self.current_route {
                match &action {
                    CreatorAction::AddPoint(point) => {
                        route.add_point(point.clone());
                    }
                    CreatorAction::RemovePoint(idx, _) => {
                        route.remove_point(*idx);
                    }
                    CreatorAction::AddObject(obj) => {
                        route.add_object(obj.clone());
                    }
                    CreatorAction::RemoveObject(obj) => {
                        route.remove_object(obj.id);
                    }
                    CreatorAction::MoveObject(id, _, new_pos) => {
                        if let Some(obj) = route.find_object_mut(*id) {
                            obj.position = *new_pos;
                        }
                    }
                    CreatorAction::MovePoint(idx, _, new) => {
                        if *idx < route.points.len() {
                            route.points[*idx] = new.clone();
                        }
                    }
                }
            }
            self.undo_stack.push(action);
        }
    }

    /// Can undo
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Can redo
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Snap position to grid if enabled
    pub fn snap_position(&self, pos: Vec3) -> Vec3 {
        if self.snap_to_grid {
            Vec3::new(
                (pos.x / self.grid_size).round() * self.grid_size,
                pos.y,
                (pos.z / self.grid_size).round() * self.grid_size,
            )
        } else {
            pos
        }
    }
}

impl Default for WorldCreator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_route() {
        let mut route = CustomRoute::new("Test Route".to_string());

        route.add_point(RoutePoint::new(45.0, 6.0, 100.0));
        route.add_point(RoutePoint::new(45.001, 6.001, 150.0));

        assert_eq!(route.points.len(), 2);
        assert!(route.total_distance() > 0.0);
        assert!((route.total_elevation_gain() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_world_creator_undo_redo() {
        let mut creator = WorldCreator::new();
        creator.new_route("Test".to_string());

        creator.add_point(RoutePoint::new(45.0, 6.0, 100.0));
        assert_eq!(creator.route().unwrap().points.len(), 1);

        creator.undo();
        assert_eq!(creator.route().unwrap().points.len(), 0);

        creator.redo();
        assert_eq!(creator.route().unwrap().points.len(), 1);
    }
}
