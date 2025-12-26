//! Creator tool implementations.

use super::{PlaceableType, PlacedObject, RoutePoint};
use glam::Vec3;

/// Tool mode for terrain editing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTool {
    /// Raise terrain
    Raise,
    /// Lower terrain
    Lower,
    /// Smooth terrain
    Smooth,
    /// Flatten terrain
    Flatten,
    /// Paint surface type
    Paint,
}

/// Brush settings for terrain tools
#[derive(Debug, Clone)]
pub struct BrushSettings {
    /// Brush radius in meters
    pub radius: f32,
    /// Brush strength (0..1)
    pub strength: f32,
    /// Falloff type
    pub falloff: BrushFalloff,
}

/// Brush falloff types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushFalloff {
    /// Constant strength across brush
    Constant,
    /// Linear falloff from center
    Linear,
    /// Smooth (cosine) falloff
    Smooth,
    /// Spherical falloff
    Spherical,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            radius: 10.0,
            strength: 0.5,
            falloff: BrushFalloff::Smooth,
        }
    }
}

impl BrushSettings {
    /// Calculate brush influence at distance from center
    pub fn influence(&self, distance: f32) -> f32 {
        if distance >= self.radius {
            return 0.0;
        }

        let normalized = distance / self.radius;
        let falloff = match self.falloff {
            BrushFalloff::Constant => 1.0,
            BrushFalloff::Linear => 1.0 - normalized,
            BrushFalloff::Smooth => ((1.0 - normalized) * std::f32::consts::PI * 0.5).cos(),
            BrushFalloff::Spherical => (1.0 - normalized * normalized).sqrt(),
        };

        falloff * self.strength
    }
}

/// Route editing tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteTool {
    /// Add points to route
    Draw,
    /// Select and move points
    Select,
    /// Insert point between existing points
    Insert,
    /// Delete points
    Delete,
    /// Smooth the route curve
    Smooth,
}

/// Object placement tool
#[derive(Debug, Clone)]
pub struct ObjectTool {
    /// Type of object to place
    pub object_type: PlaceableType,
    /// Random rotation enabled
    pub random_rotation: bool,
    /// Random scale range
    pub scale_range: (f32, f32),
    /// Random variant enabled
    pub random_variant: bool,
    /// Number of variants available
    pub variant_count: u8,
}

impl ObjectTool {
    /// Create tool for object type
    pub fn new(object_type: PlaceableType) -> Self {
        let (scale_range, variant_count) = match object_type {
            PlaceableType::Tree => ((0.8, 1.5), 8),
            PlaceableType::Rock => ((0.3, 2.0), 6),
            PlaceableType::Building => ((0.8, 1.2), 4),
            PlaceableType::Sign => ((0.8, 1.2), 3),
            PlaceableType::Barrier => ((1.0, 1.0), 2),
            PlaceableType::Light => ((0.9, 1.1), 2),
            PlaceableType::Spectators => ((0.8, 1.2), 4),
            PlaceableType::Marker => ((0.5, 1.5), 1),
        };

        Self {
            object_type,
            random_rotation: matches!(
                object_type,
                PlaceableType::Tree | PlaceableType::Rock | PlaceableType::Spectators
            ),
            scale_range,
            random_variant: variant_count > 1,
            variant_count,
        }
    }

    /// Create object at position with tool settings
    pub fn create_object(&self, position: Vec3) -> PlacedObject {
        let mut obj = PlacedObject::new(self.object_type, position);

        if self.random_rotation {
            // Random rotation 0..2π
            obj.rotation = rand_simple() * std::f32::consts::TAU;
        }

        // Random scale within range
        let scale_t = rand_simple();
        obj.scale = self.scale_range.0 + (self.scale_range.1 - self.scale_range.0) * scale_t;

        if self.random_variant {
            obj.variant = (rand_simple() * self.variant_count as f32) as u8;
        }

        obj
    }
}

/// Route smoothing parameters
#[derive(Debug, Clone)]
pub struct SmoothingParams {
    /// Number of smoothing iterations
    pub iterations: u32,
    /// Smoothing factor (0..1, higher = smoother)
    pub factor: f32,
    /// Whether to preserve endpoints
    pub preserve_ends: bool,
}

impl Default for SmoothingParams {
    fn default() -> Self {
        Self {
            iterations: 3,
            factor: 0.5,
            preserve_ends: true,
        }
    }
}

/// Smooth a route using Chaikin's algorithm variant
pub fn smooth_route(points: &mut Vec<RoutePoint>, params: &SmoothingParams) {
    if points.len() < 3 {
        return;
    }

    for _ in 0..params.iterations {
        let mut new_points = Vec::with_capacity(points.len());

        if params.preserve_ends {
            new_points.push(points[0].clone());
        }

        for i in 0..points.len() - 1 {
            let p1 = &points[i];
            let p2 = &points[i + 1];

            // Create two intermediate points
            let t1 = 0.25 + (1.0 - params.factor) * 0.25;
            let t2 = 0.75 - (1.0 - params.factor) * 0.25;

            let q = lerp_point(p1, p2, t1);
            let r = lerp_point(p1, p2, t2);

            if !params.preserve_ends || i > 0 {
                new_points.push(q);
            }
            new_points.push(r);
        }

        if params.preserve_ends {
            new_points.push(points.last().unwrap().clone());
        }

        *points = new_points;
    }
}

/// Lerp between two route points
fn lerp_point(a: &RoutePoint, b: &RoutePoint, t: f32) -> RoutePoint {
    RoutePoint {
        latitude: a.latitude + (b.latitude - a.latitude) * t as f64,
        longitude: a.longitude + (b.longitude - a.longitude) * t as f64,
        elevation: a.elevation + (b.elevation - a.elevation) * t,
        road_width: a.road_width + (b.road_width - a.road_width) * t,
        surface: a.surface, // Use first point's surface
    }
}

/// Calculate optimal point insertion index
pub fn find_insertion_index(points: &[RoutePoint], lat: f64, lon: f64) -> usize {
    if points.len() < 2 {
        return points.len();
    }

    let mut min_distance = f64::MAX;
    let mut best_index = 1;

    for i in 0..points.len() - 1 {
        let p1 = &points[i];
        let p2 = &points[i + 1];

        // Distance from point to line segment
        let dist = point_to_segment_distance(
            lat,
            lon,
            p1.latitude,
            p1.longitude,
            p2.latitude,
            p2.longitude,
        );

        if dist < min_distance {
            min_distance = dist;
            best_index = i + 1;
        }
    }

    best_index
}

/// Calculate distance from point to line segment
fn point_to_segment_distance(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;

    if dx == 0.0 && dy == 0.0 {
        // Segment is a point
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }

    let t = ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy);
    let t = t.clamp(0.0, 1.0);

    let nearest_x = x1 + t * dx;
    let nearest_y = y1 + t * dy;

    ((px - nearest_x).powi(2) + (py - nearest_y).powi(2)).sqrt()
}

/// Simple random number (0.0-1.0)
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / 4_294_967_295.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brush_influence() {
        let brush = BrushSettings {
            radius: 10.0,
            strength: 1.0,
            falloff: BrushFalloff::Linear,
        };

        // At center, full influence
        assert!((brush.influence(0.0) - 1.0).abs() < 0.01);

        // At edge, zero influence
        assert!((brush.influence(10.0) - 0.0).abs() < 0.01);

        // At half radius, half influence (linear)
        assert!((brush.influence(5.0) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_object_tool() {
        let tool = ObjectTool::new(PlaceableType::Tree);
        let obj = tool.create_object(Vec3::new(100.0, 0.0, 200.0));

        assert_eq!(obj.object_type, PlaceableType::Tree);
        assert!(obj.scale >= 0.8 && obj.scale <= 1.5);
    }

    #[test]
    fn test_smooth_route() {
        use crate::world::procedural::SurfaceType;

        let mut points = vec![
            RoutePoint {
                latitude: 0.0,
                longitude: 0.0,
                elevation: 0.0,
                road_width: 6.0,
                surface: SurfaceType::Asphalt,
            },
            RoutePoint {
                latitude: 1.0,
                longitude: 0.0,
                elevation: 10.0,
                road_width: 6.0,
                surface: SurfaceType::Asphalt,
            },
            RoutePoint {
                latitude: 1.0,
                longitude: 1.0,
                elevation: 5.0,
                road_width: 6.0,
                surface: SurfaceType::Asphalt,
            },
        ];

        let original_len = points.len();
        smooth_route(&mut points, &SmoothingParams::default());

        // Smoothing should add points
        assert!(points.len() > original_len);
    }
}
