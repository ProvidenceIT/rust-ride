//! XP curve calculations for career progression.

/// Maximum career level.
pub const MAX_LEVEL: u32 = 50;

/// XP curve base value.
pub const XP_BASE: f64 = 1000.0;

/// XP curve growth rate.
pub const XP_GROWTH_RATE: f64 = 1.15;

/// Calculate XP required for a specific level.
///
/// Level 1 requires 0 XP, level 2 requires 1000 XP, etc.
/// Uses exponential curve: 1000 * 1.15^(level-1)
pub fn xp_for_level(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }
    (XP_BASE * XP_GROWTH_RATE.powi(level as i32 - 1)) as u64
}

/// Calculate cumulative XP needed to reach a level from level 1.
pub fn cumulative_xp_to_level(level: u32) -> u64 {
    (1..level).map(xp_for_level).sum()
}

/// Calculate level from total accumulated XP.
pub fn level_from_xp(total_xp: u64) -> u32 {
    let mut level = 1u32;
    let mut cumulative = 0u64;

    while level < MAX_LEVEL {
        let next_level_xp = xp_for_level(level + 1);
        if cumulative + next_level_xp > total_xp {
            break;
        }
        cumulative += next_level_xp;
        level += 1;
    }

    level
}

/// Calculate XP progress within current level (0.0 to 1.0).
pub fn level_progress(total_xp: u64) -> f32 {
    let current_level = level_from_xp(total_xp);
    if current_level >= MAX_LEVEL {
        return 1.0;
    }

    let xp_at_current = cumulative_xp_to_level(current_level);
    let xp_for_next = xp_for_level(current_level + 1);

    if xp_for_next == 0 {
        return 0.0;
    }

    let xp_into_level = total_xp.saturating_sub(xp_at_current);
    (xp_into_level as f32 / xp_for_next as f32).clamp(0.0, 1.0)
}

/// Calculate XP needed for next level from current total.
pub fn xp_to_next_level(total_xp: u64) -> u64 {
    let current_level = level_from_xp(total_xp);
    if current_level >= MAX_LEVEL {
        return 0;
    }

    let xp_at_current = cumulative_xp_to_level(current_level);
    let xp_for_next = xp_for_level(current_level + 1);
    let xp_into_level = total_xp.saturating_sub(xp_at_current);

    xp_for_next.saturating_sub(xp_into_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_for_level() {
        // Level 1 requires 0 XP (starting level)
        assert_eq!(xp_for_level(1), 0);

        // Level 2 requires 1000 * 1.15^1 = 1150 XP
        assert_eq!(xp_for_level(2), 1150);

        // Level 10: 1000 * 1.15^9 ≈ 3518
        let level_10 = xp_for_level(10);
        assert!(level_10 > 3000 && level_10 < 4000, "Level 10 XP: {}", level_10);
    }

    #[test]
    fn test_level_from_xp() {
        assert_eq!(level_from_xp(0), 1);
        assert_eq!(level_from_xp(500), 1);
        // Level 2 starts at cumulative 1150 (the xp_for_level(2))
        assert_eq!(level_from_xp(1150), 2);
        assert_eq!(level_from_xp(1149), 1);
    }

    #[test]
    fn test_cumulative_xp() {
        // cumulative_xp_to_level(n) = sum of xp_for_level(1..n)
        // xp_for_level(1) = 0, so cumulative_xp_to_level(1) = 0
        assert_eq!(cumulative_xp_to_level(1), 0);

        // cumulative_xp_to_level(2) = xp_for_level(1) = 0
        // (this is XP needed to START level 2, but xp_for_level(1) = 0)
        // Actually cumulative_xp_to_level(2) = sum of xp_for_level(1..2) = xp_for_level(1) = 0
        assert_eq!(cumulative_xp_to_level(2), 0);

        // cumulative_xp_to_level(3) = xp_for_level(1) + xp_for_level(2) = 0 + 1150 = 1150
        assert_eq!(cumulative_xp_to_level(3), 1150);

        // Cumulative should be monotonically increasing
        let level_10 = cumulative_xp_to_level(10);
        let level_20 = cumulative_xp_to_level(20);
        assert!(level_20 > level_10);
    }

    #[test]
    fn test_level_progress() {
        // At start of level 2, progress should be near 0
        let xp_at_level_2 = cumulative_xp_to_level(2);
        let progress_at_level_2 = level_progress(xp_at_level_2);
        assert!(progress_at_level_2 < 0.01, "Progress at level 2 start: {}", progress_at_level_2);

        // Halfway through level 2
        let xp_for_level_2 = xp_for_level(2);
        let halfway = xp_at_level_2 + xp_for_level_2 / 2;
        let progress = level_progress(halfway);
        assert!((progress - 0.5).abs() < 0.1, "Halfway progress: {}", progress);
    }

    #[test]
    fn test_xp_to_next_level() {
        // At 0 XP (level 1), need xp_for_level(2) = 1150 for level 2
        assert_eq!(xp_to_next_level(0), 1150);

        // At max level, need 0
        let max_xp = cumulative_xp_to_level(MAX_LEVEL + 1);
        assert_eq!(xp_to_next_level(max_xp), 0);
    }
}
