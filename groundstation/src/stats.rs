pub fn median(values: &[u64]) -> Option<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len() / 2).copied()
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
        assert_eq!((shown(Some(7)), shown(None)), ("7".to_string(), "none".to_string()));
    }
}
