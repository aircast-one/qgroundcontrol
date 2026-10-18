pub fn median(values: &[u64]) -> Option<u64> {
    percentile(values, 0.5)
}

pub fn percentile(values: &[u64], fraction: f64) -> Option<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let at = ((sorted.len() as f64 * fraction) as usize).min(sorted.len().saturating_sub(1));
    sorted.get(at).copied()
}

pub fn shown(value: Option<u64>) -> String {
    value.map_or_else(|| "none".to_string(), |ms| ms.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_median_is_the_middle_value_whatever_the_order() {
        assert_eq!(median(&[40, 10, 30, 20, 50]), Some(30));
        assert_eq!(median(&[]), None);
        assert_eq!(
            (shown(Some(7)), shown(None)),
            ("7".to_string(), "none".to_string())
        );
        assert_eq!(percentile(&(1..=10).collect::<Vec<_>>(), 0.9), Some(10));
        assert_eq!(percentile(&[5], 0.9), Some(5));
        assert_eq!(percentile(&[], 0.9), None);
    }
}
