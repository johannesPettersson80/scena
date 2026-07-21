use std::collections::BTreeSet;

/// Returns deterministic nearest names after case and separator normalization.
///
/// Results are ordered by edit distance, then normalized name, then original
/// name. Exact duplicate candidates are removed and at most `limit` entries are
/// returned.
pub fn nearest_name_candidates<I, S>(input: &str, candidates: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if limit == 0 {
        return Vec::new();
    }
    let normalized_input = normalize_name(input);
    let mut unique = BTreeSet::new();
    for candidate in candidates {
        unique.insert(candidate.as_ref().to_owned());
    }
    let mut ranked = unique
        .into_iter()
        .map(|candidate| {
            let normalized = normalize_name(&candidate);
            let distance = edit_distance(&normalized_input, &normalized);
            (distance, normalized, candidate)
        })
        .collect::<Vec<_>>();
    ranked.sort();
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, _, candidate)| candidate)
        .collect()
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right_chars.len()]
}
