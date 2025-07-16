const ADJACENCY_RULES: &[&[bool]] = &[
    &[false, false, false, false, true, true, false, true, true],
    &[false, true, true, false, true, true, false, true, true],
    &[false, true, true, false, true, true, false, false, false],
    &[false, true, true, true, true, true, true, true, true],
    &[true, true, true, true, true, true, false, true, true],
    &[false, false, false, true, true, true, true, true, true],
    &[true, true, true, true, true, true, true, true, true],
    &[true, true, true, true, true, true, false, false, false],
    &[true, true, false, true, true, true, true, true, true],
    &[true, true, true, true, true, true, true, true, false],
    &[false, false, false, true, true, false, true, true, false],
    &[true, true, false, true, true, false, true, true, false],
    &[true, true, false, true, true, false, false, false, false],
    &[false, false, false, false, true, false, false, false, false],
    &[false, true, false, true, true, true, false, true, false],
    &[false, false, false, false, true, true, false, true, false],
    &[false, true, false, false, true, true, false, false, false],
    &[false, false, false, false, true, false, false, true, false],
    &[false, true, false, false, true, false, false, true, false],
    &[false, true, false, false, true, false, false, false, false],
    &[false, false, false, true, true, false, false, true, false],
    &[false, true, false, true, true, false, false, false, false],
    &[false, true, true, true, true, true, true, true, false],
    &[true, true, false, true, true, true, false, true, true],
    &[false, false, false, false, true, true, false, false, false],
    &[false, false, false, true, true, true, false, true, true],
    &[false, true, true, true, true, true, false, false, false],
    &[false, true, false, false, true, true, false, true, true],
    &[false, true, true, false, true, true, false, true, false],
    &[false, false, false, true, true, true, false, false, false],
    &[false, false, false, true, true, true, true, true, false],
    &[true, true, false, true, true, true, false, false, false],
    &[false, true, false, true, true, false, true, true, false],
    &[true, true, false, true, true, false, false, true, false],
    &[false, false, false, true, true, false, false, false, false],
    &[false, true, true, true, true, true, false, true, true],
    &[true, true, true, true, true, true, false, true, false],
    &[false, true, false, false, true, true, false, true, false],
    &[false, true, false, true, true, true, false, false, false],
    &[true, true, false, true, true, true, true, true, false],
    &[false, true, false, true, true, true, true, true, true],
    &[false, false, false, true, true, true, false, true, false],
    &[false, true, false, true, true, false, false, true, false],
    &[false, true, false, true, true, true, false, true, true],
    &[false, true, true, true, true, true, false, true, false],
    &[false, true, false, true, true, true, true, true, false],
    &[true, true, false, true, true, true, false, true, false],
];

pub fn rule_complexity(rule: &[bool]) -> usize {
    rule.iter().filter(|&&x| x).count()
}

/// Returns the index of the matching rule with most complexity.
pub fn match_rule(neighborhood: &[bool; 9]) -> Option<usize> {
    let mut max_complexity = 0;
    let mut best_match = None;
    for (i, rule) in ADJACENCY_RULES.iter().enumerate() {
        let as_vec = rule.to_vec();
        if neighborhood.iter().zip(as_vec).all(|(&n, r)| n == r) {
            let complexity = rule_complexity(rule);
            if complexity > max_complexity {
                max_complexity = complexity;
                best_match = Some(i);
            }
        }
    }
    best_match
}
