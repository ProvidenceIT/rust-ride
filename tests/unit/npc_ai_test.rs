//! Unit tests for NPC AI behavior
//!
//! T055: Unit test for NPC AI behavior in tests/unit/npc_ai_test.rs

use rustride::world::npc::ai::{calculate_speed, calculate_speed_physics, AiBehavior};
use rustride::world::npc::{NpcCyclist, NpcDifficulty, NpcManager, NpcSettings};

/// Test AI behavior creation
#[test]
fn test_ai_behavior_default() {
    let behavior = AiBehavior::default();
    assert!((behavior.power_variation_percent - 10.0).abs() < 0.01);
    assert!((behavior.variation_interval - 5.0).abs() < 0.01);
    assert!(behavior.gradient_response);
}

/// Test AI behavior updates NPC
#[test]
fn test_ai_behavior_update() {
    let mut behavior = AiBehavior::default();
    let mut npc = NpcCyclist::new(1, "Test".to_string(), 0.0, 200, 0);

    // Initial power
    let _initial_power = npc.current_power_watts;

    // Update with 6 seconds (exceeds variation interval)
    behavior.update(&mut npc, 6.0, 0.0);

    // Power should have potentially changed
    // Note: due to randomness, we just check it's in a reasonable range
    assert!(npc.current_power_watts > 100 && npc.current_power_watts < 400);
}

/// Test calculate_speed on flat terrain
#[test]
fn test_calculate_speed_flat() {
    let speed = calculate_speed(200, 0.0, 75.0);

    // At 200W on flat, expect roughly 8-12 m/s (28-43 km/h)
    assert!(speed > 5.0, "Speed should be > 5 m/s, got {}", speed);
    assert!(speed < 15.0, "Speed should be < 15 m/s, got {}", speed);
}

/// Test calculate_speed on uphill
#[test]
fn test_calculate_speed_uphill() {
    let flat_speed = calculate_speed(200, 0.0, 75.0);
    let uphill_speed = calculate_speed(200, 8.0, 75.0);

    assert!(
        uphill_speed < flat_speed,
        "Uphill speed {} should be less than flat speed {}",
        uphill_speed,
        flat_speed
    );
}

/// Test calculate_speed on downhill
#[test]
fn test_calculate_speed_downhill() {
    let flat_speed = calculate_speed(200, 0.0, 75.0);
    let downhill_speed = calculate_speed(200, -8.0, 75.0);

    assert!(
        downhill_speed > flat_speed,
        "Downhill speed {} should be greater than flat speed {}",
        downhill_speed,
        flat_speed
    );
}

/// Test speed increases with power
#[test]
fn test_calculate_speed_power_increase() {
    let low_power_speed = calculate_speed(150, 0.0, 75.0);
    let high_power_speed = calculate_speed(300, 0.0, 75.0);

    assert!(
        high_power_speed > low_power_speed,
        "Higher power {} should give faster speed than lower power {}",
        high_power_speed,
        low_power_speed
    );
}

/// Test physics-based speed calculation
#[test]
fn test_calculate_speed_physics_flat() {
    // Typical cycling parameters
    let cda = 0.3; // m² (typical road cyclist)
    let crr = 0.004; // Rolling resistance coefficient

    let speed = calculate_speed_physics(200.0, 0.0, 75.0, cda, crr);

    // At 200W on flat, expect roughly 8-10 m/s (29-36 km/h)
    assert!(speed > 6.0, "Speed should be > 6 m/s, got {}", speed);
    assert!(speed < 12.0, "Speed should be < 12 m/s, got {}", speed);
}

/// Test physics calculation on steep climb
#[test]
fn test_calculate_speed_physics_steep_climb() {
    let cda = 0.3;
    let crr = 0.004;

    let flat_speed = calculate_speed_physics(200.0, 0.0, 75.0, cda, crr);
    let climb_speed = calculate_speed_physics(200.0, 10.0, 75.0, cda, crr);

    assert!(
        climb_speed < flat_speed / 2.0,
        "10% climb speed {} should be less than half of flat speed {}",
        climb_speed,
        flat_speed
    );
}

/// Test physics calculation with higher power on climb
#[test]
fn test_calculate_speed_physics_power_on_climb() {
    let cda = 0.3;
    let crr = 0.004;

    let low_power = calculate_speed_physics(200.0, 8.0, 75.0, cda, crr);
    let high_power = calculate_speed_physics(300.0, 8.0, 75.0, cda, crr);

    assert!(
        high_power > low_power,
        "Higher power {} should be faster than lower power {} on climb",
        high_power,
        low_power
    );
}

/// Test speed limits are enforced
#[test]
fn test_calculate_speed_limits() {
    // Very low power shouldn't go below minimum
    let low_speed = calculate_speed(50, 10.0, 75.0);
    assert!(low_speed >= 1.0, "Speed should not go below 1 m/s");

    // Very high power on descent shouldn't exceed maximum
    let high_speed = calculate_speed(500, -15.0, 75.0);
    assert!(high_speed <= 25.0, "Speed should not exceed 25 m/s");
}

/// Test gradient response affects power
#[test]
fn test_ai_gradient_response() {
    let mut npc = NpcCyclist::new(1, "Test".to_string(), 0.0, 200, 0);

    // Manually test gradient effect on update
    npc.update(1.0, 5.0); // 5% climb

    // NPC should still function
    assert!(npc.speed_mps > 0.0);
    assert!(npc.distance_meters > 0.0);
}

/// Test NPC cyclist basic functionality
#[test]
fn test_npc_cyclist_creation() {
    let npc = NpcCyclist::new(42, "Pro Rider".to_string(), 1000.0, 300, 3);

    assert_eq!(npc.id, 42);
    assert_eq!(npc.name, "Pro Rider");
    assert!((npc.distance_meters - 1000.0).abs() < 0.01);
    assert_eq!(npc.target_power_watts, 300);
    assert_eq!(npc.appearance_index, 3);
    assert!(!npc.passed_by_user);
    assert!(!npc.user_drafting);
}

/// Test NPC update moves position
#[test]
fn test_npc_update_moves_position() {
    let mut npc = NpcCyclist::new(1, "Test".to_string(), 100.0, 200, 0);
    let initial_distance = npc.distance_meters;

    // Update for 1 second on flat
    npc.update(1.0, 0.0);

    assert!(
        npc.distance_meters > initial_distance,
        "NPC should have moved forward"
    );
    assert!(npc.speed_mps > 0.0, "NPC should have positive speed");
}

/// Test NPC slower on climbs
#[test]
fn test_npc_slower_on_climb() {
    let mut npc_flat = NpcCyclist::new(1, "Flat".to_string(), 0.0, 200, 0);
    let mut npc_climb = NpcCyclist::new(2, "Climb".to_string(), 0.0, 200, 0);

    // Update both for same time
    npc_flat.update(5.0, 0.0);
    npc_climb.update(5.0, 10.0);

    assert!(
        npc_flat.distance_meters > npc_climb.distance_meters,
        "Flat NPC {} should travel further than climbing NPC {}",
        npc_flat.distance_meters,
        npc_climb.distance_meters
    );
}

/// Test difficulty multiplier values
#[test]
fn test_difficulty_multipliers() {
    assert!((NpcDifficulty::Easy.ftp_multiplier() - 0.5).abs() < 0.01);
    assert!((NpcDifficulty::Medium.ftp_multiplier() - 0.8).abs() < 0.01);
    assert!((NpcDifficulty::MatchUser.ftp_multiplier() - 1.0).abs() < 0.01);
    assert!((NpcDifficulty::Hard.ftp_multiplier() - 1.1).abs() < 0.01);
    assert!((NpcDifficulty::VeryHard.ftp_multiplier() - 1.3).abs() < 0.01);
}

/// Test NPC manager creation
#[test]
fn test_npc_manager_creation() {
    let settings = NpcSettings::default();
    let manager = NpcManager::new(settings.clone(), 250);

    assert!(manager.npcs().is_empty());
    assert_eq!(manager.stats().npcs_passed, 0);
}

/// Test NPC manager spawning
#[test]
fn test_npc_manager_spawn() {
    let settings = NpcSettings {
        enabled: true,
        count: 8,
        difficulty: NpcDifficulty::Medium,
        show_names: true,
    };

    let mut manager = NpcManager::new(settings, 250);
    manager.spawn_for_route(10000.0);

    assert_eq!(manager.npcs().len(), 8);
}

/// Test NPC manager disabled
#[test]
fn test_npc_manager_disabled() {
    let settings = NpcSettings {
        enabled: false,
        count: 10,
        difficulty: NpcDifficulty::Medium,
        show_names: true,
    };

    let mut manager = NpcManager::new(settings, 250);
    manager.spawn_for_route(10000.0);

    assert!(
        manager.npcs().is_empty(),
        "No NPCs should spawn when disabled"
    );
}
