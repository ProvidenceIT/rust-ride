//! Unit tests for NPC spawner
//!
//! T056: Unit test for NPC spawner in tests/unit/npc_spawner_test.rs

use rustride::world::npc::spawner::{
    generate_spawn_positions, NpcAppearance, NpcSpawner, SpawnStrategy,
};
use rustride::world::npc::NpcDifficulty;

/// Test NPC appearance from index
#[test]
fn test_appearance_from_index() {
    let appearance = NpcAppearance::from_index(0);

    assert!(!appearance.name.is_empty());
    // Blue team
    assert_eq!(appearance.jersey_color, [0, 100, 200]);
}

/// Test all appearance variants
#[test]
fn test_all_appearance_variants() {
    for i in 0..8 {
        let appearance = NpcAppearance::from_index(i);
        assert!(
            !appearance.name.is_empty(),
            "Appearance {} should have name",
            i
        );
        // All appearances should have valid colors
        assert!(appearance.jersey_color[0] <= 255);
        assert!(appearance.jersey_color[1] <= 255);
        assert!(appearance.jersey_color[2] <= 255);
    }
}

/// Test appearance wraps around
#[test]
fn test_appearance_index_wrapping() {
    let appearance_0 = NpcAppearance::from_index(0);
    let appearance_8 = NpcAppearance::from_index(8); // Should wrap to 0

    assert_eq!(appearance_0.jersey_color, appearance_8.jersey_color);
}

/// Test NPC spawner creation
#[test]
fn test_spawner_creation() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
    let target_power = spawner.target_power();

    // 250 * 0.8 = 200
    assert_eq!(target_power, 200);
}

/// Test spawner with different difficulties
#[test]
fn test_spawner_difficulty_power() {
    let route_length = 10000.0;
    let user_ftp = 250u16;

    let easy = NpcSpawner::new(user_ftp, NpcDifficulty::Easy, route_length);
    let medium = NpcSpawner::new(user_ftp, NpcDifficulty::Medium, route_length);
    let hard = NpcSpawner::new(user_ftp, NpcDifficulty::Hard, route_length);
    let very_hard = NpcSpawner::new(user_ftp, NpcDifficulty::VeryHard, route_length);

    assert_eq!(easy.target_power(), 125); // 250 * 0.5
    assert_eq!(medium.target_power(), 200); // 250 * 0.8
    assert_eq!(hard.target_power(), 275); // 250 * 1.1
    assert_eq!(very_hard.target_power(), 325); // 250 * 1.3
}

/// Test spawner spawns correct count
#[test]
fn test_spawner_spawn_count() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);

    let npcs_5 = spawner.spawn(5);
    let npcs_10 = spawner.spawn(10);
    let npcs_20 = spawner.spawn(20);

    assert_eq!(npcs_5.len(), 5);
    assert_eq!(npcs_10.len(), 10);
    assert_eq!(npcs_20.len(), 20);
}

/// Test spawned NPCs have unique IDs
#[test]
fn test_spawned_npcs_unique_ids() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
    let npcs = spawner.spawn(10);

    let ids: Vec<u32> = npcs.iter().map(|n| n.id).collect();
    let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();

    assert_eq!(unique_count, 10, "All NPC IDs should be unique");
}

/// Test spawned NPCs have varied positions
#[test]
fn test_spawned_npcs_varied_positions() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
    let npcs = spawner.spawn(5);

    // Check NPCs are spread out
    for i in 0..npcs.len() - 1 {
        assert_ne!(
            npcs[i].distance_meters,
            npcs[i + 1].distance_meters,
            "NPCs should have different positions"
        );
    }
}

/// Test spawned NPCs are in first half of route
#[test]
fn test_spawned_npcs_in_first_half() {
    let route_length = 10000.0;
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, route_length);
    let npcs = spawner.spawn(10);

    for npc in &npcs {
        assert!(
            npc.distance_meters >= 0.0,
            "NPC position should be positive"
        );
        assert!(
            npc.distance_meters <= route_length * 0.7,
            "NPC position {} should be in first ~50% of route (with variation)",
            npc.distance_meters
        );
    }
}

/// Test random appearance generation
#[test]
fn test_random_appearance() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);

    // Generate a few random appearances
    for _ in 0..10 {
        let appearance = spawner.random_appearance();
        assert!(!appearance.name.is_empty());
    }
}

/// Test even distribution spawn strategy
#[test]
fn test_spawn_strategy_even() {
    let positions = generate_spawn_positions(4, 10000.0, 0.0, SpawnStrategy::EvenDistribution);

    assert_eq!(positions.len(), 4);

    // Should be roughly evenly spaced in first 50%
    for i in 0..positions.len() - 1 {
        assert!(
            positions[i] < positions[i + 1],
            "Positions should be in order"
        );
    }

    // All should be within first 50%
    for pos in &positions {
        assert!(*pos <= 5000.0, "Even distribution should be in first 50%");
    }
}

/// Test near user spawn strategy
#[test]
fn test_spawn_strategy_near_user() {
    let user_start = 1000.0;
    let positions = generate_spawn_positions(6, 10000.0, user_start, SpawnStrategy::NearUser);

    assert_eq!(positions.len(), 6);

    // All should be near user start
    for pos in &positions {
        let distance_from_user = (pos - user_start).abs();
        assert!(
            distance_from_user <= 150.0,
            "Position {} should be within 150m of user {}",
            pos,
            user_start
        );
    }
}

/// Test random spawn strategy
#[test]
fn test_spawn_strategy_random() {
    let positions = generate_spawn_positions(5, 10000.0, 0.0, SpawnStrategy::Random);

    assert_eq!(positions.len(), 5);

    // All should be within first 50%
    for pos in &positions {
        assert!(*pos >= 0.0);
        assert!(*pos <= 5000.0, "Random spawn should be in first 50%");
    }
}

/// Test peloton spawn strategy
#[test]
fn test_spawn_strategy_peloton() {
    let user_start = 0.0;
    let positions = generate_spawn_positions(6, 10000.0, user_start, SpawnStrategy::Peloton);

    assert_eq!(positions.len(), 6);

    // Should be clustered together
    let min_pos = positions.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_pos = positions.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    assert!(
        max_pos - min_pos < 30.0,
        "Peloton should be clustered within 30m"
    );
}

/// Test NPC power varies from target
#[test]
fn test_spawned_npc_power_variation() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
    let target_power = spawner.target_power();

    // Spawn multiple times and check for variation
    let mut saw_variation = false;
    for _ in 0..10 {
        let npcs = spawner.spawn(5);
        for npc in &npcs {
            if npc.target_power_watts != target_power {
                saw_variation = true;
                break;
            }
        }
    }

    // There should be some variation (not all exactly target)
    // Note: This may occasionally fail due to randomness, but unlikely
    assert!(
        saw_variation,
        "NPC power should have some variation from target"
    );
}

/// Test spawner with zero count
#[test]
fn test_spawner_zero_count() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
    let npcs = spawner.spawn(0);

    assert!(npcs.is_empty());
}

/// Test spawner with very short route
#[test]
fn test_spawner_short_route() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 100.0);
    let npcs = spawner.spawn(3);

    assert_eq!(npcs.len(), 3);

    // All should fit in the short route
    for npc in &npcs {
        assert!(npc.distance_meters <= 100.0);
    }
}

/// Test appearance cycling
#[test]
fn test_spawned_npc_appearance_cycling() {
    let spawner = NpcSpawner::new(250, NpcDifficulty::Medium, 10000.0);
    let npcs = spawner.spawn(16);

    // With 16 NPCs and 8 appearances, should see cycling
    let mut appearances = std::collections::HashSet::new();
    for npc in &npcs {
        appearances.insert(npc.appearance_index);
    }

    // Should have all 8 appearances represented
    assert_eq!(appearances.len(), 8);
}
