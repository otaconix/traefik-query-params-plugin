pub fn position_to_index<T>(position: i8, vec: &[T], for_insertion: bool) -> usize {
    if position < 0 {
        vec.len()
            .checked_sub(position.unsigned_abs() as usize)
            .map(|i| if for_insertion { i + 1 } else { i })
            .unwrap_or(0)
    } else {
        (position as usize).min(vec.len())
    }
}
