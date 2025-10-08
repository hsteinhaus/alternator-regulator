use defmt::debug;

/// Detects zero crossings with hysteresis to prevent noise-induced oscillations.
///
/// # Arguments
/// * `value` - The current value to check
/// * `threshold` - The zero crossing point
/// * `hysteresis` - The hysteresis factor (0.05 ->  dead band of +-5%)
/// * `last_state` - The previous state (true = above, false = below)
///
/// # Returns
/// * `(new_state, crossed)` - The new state and whether a crossing occurred
///
/// # Example
/// ```
/// let mut state = false;
/// let (new_state, crossed) = detect_zero_crossing_with_hysteresis(1.5, 0.0, 0.5, state);
/// state = new_state;
/// if crossed {
///     // Handle crossing event
/// }
/// ```
pub fn detect_zero_crossing_with_hysteresis(
    value: f32,
    threshold: f32,
    hysteresis: f32,
    last_state: bool,
) -> (bool, bool) {
    let upper_threshold = threshold * (1. + hysteresis);
    let lower_threshold = threshold * (1. - hysteresis);
    debug!("deadband: {}..{}", lower_threshold, upper_threshold);
    let new_state = if last_state {
        // Currently above: need to cross below the lower threshold to change state
        value >= lower_threshold
    } else {
        // Currently below: need to cross above the upper threshold to change state
        value >= upper_threshold
    };

    let crossed = new_state != last_state;
    (new_state, crossed)
}

#[cfg(all(test, not(target_arch = "xtensa"), not(target_arch = "riscv32")))]
mod tests {
    use super::*;

    #[test]
    fn test_zero_crossing_basic() {
        let threshold = 0.0;
        let hysteresis = 0.05;

        // Start below threshold
        let mut state = false;

        // Move up but stay in hysteresis band - no crossing
        let (new_state, crossed) = detect_zero_crossing_with_hysteresis(0.3, threshold, hysteresis, state);
        assert_eq!(new_state, false);
        assert_eq!(crossed, false);
        state = new_state;

        // Cross above upper threshold - crossing detected
        let (new_state, crossed) = detect_zero_crossing_with_hysteresis(0.6, threshold, hysteresis, state);
        assert_eq!(new_state, true);
        assert_eq!(crossed, true);
        state = new_state;

        // Stay above - no crossing
        let (new_state, crossed) = detect_zero_crossing_with_hysteresis(1.0, threshold, hysteresis, state);
        assert_eq!(new_state, true);
        assert_eq!(crossed, false);
        state = new_state;

        // Move down but stay in hysteresis band - no crossing
        let (new_state, crossed) = detect_zero_crossing_with_hysteresis(0.2, threshold, hysteresis, state);
        assert_eq!(new_state, true);
        assert_eq!(crossed, false);
        state = new_state;

        // Cross below lower threshold - crossing detected
        let (new_state, crossed) = detect_zero_crossing_with_hysteresis(-0.6, threshold, hysteresis, state);
        assert_eq!(new_state, false);
        assert_eq!(crossed, true);
    }
}
