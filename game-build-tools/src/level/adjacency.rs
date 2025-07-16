const ADJACENCY_RULES: &[&[u8]] = &[
    &[0, 0, 0, 0, 1, 1, 0, 1, 1],
    &[0, 1, 1, 0, 1, 1, 0, 1, 1],
    &[0, 1, 1, 0, 1, 1, 0, 0, 0],
    &[0, 1, 1, 1, 1, 1, 1, 1, 1],
    &[1, 1, 1, 1, 1, 1, 0, 1, 1],
    &[0, 0, 0, 1, 1, 1, 1, 1, 1],
    &[1, 1, 1, 1, 1, 1, 1, 1, 1],
    &[1, 1, 1, 1, 1, 1, 0, 0, 0],
    &[1, 1, 0, 1, 1, 1, 1, 1, 1],
    &[1, 1, 1, 1, 1, 1, 1, 1, 0],
    &[0, 0, 0, 1, 1, 0, 1, 1, 0],
    &[1, 1, 0, 1, 1, 0, 1, 1, 0],
    &[1, 1, 0, 1, 1, 0, 0, 0, 0],
    &[0, 0, 0, 0, 1, 0, 0, 0, 0],
    &[0, 1, 0, 1, 1, 1, 0, 1, 0],
    &[0, 0, 0, 0, 1, 1, 0, 1, 0],
    &[0, 1, 0, 0, 1, 1, 0, 0, 0],
    &[0, 0, 0, 0, 1, 0, 0, 1, 0],
    &[0, 1, 0, 0, 1, 0, 0, 1, 0],
    &[0, 1, 0, 0, 1, 0, 0, 0, 0],
    &[0, 0, 0, 1, 1, 0, 0, 1, 0],
    &[0, 1, 0, 1, 1, 0, 0, 0, 0],
    &[0, 1, 1, 1, 1, 1, 1, 1, 0],
    &[1, 1, 0, 1, 1, 1, 0, 1, 1],
    &[0, 0, 0, 0, 1, 1, 0, 0, 0],
    &[0, 0, 0, 1, 1, 1, 0, 1, 1],
    &[0, 1, 1, 1, 1, 1, 0, 0, 0],
    &[0, 1, 0, 0, 1, 1, 0, 1, 1],
    &[0, 1, 1, 0, 1, 1, 0, 1, 0],
    &[0, 0, 0, 1, 1, 1, 0, 0, 0],
    &[0, 0, 0, 1, 1, 1, 1, 1, 0],
    &[1, 1, 0, 1, 1, 1, 0, 0, 0],
    &[0, 1, 0, 1, 1, 0, 1, 1, 0],
    &[1, 1, 0, 1, 1, 0, 0, 1, 0],
    &[0, 0, 0, 1, 1, 0, 0, 0, 0],
    &[0, 1, 1, 1, 1, 1, 0, 1, 1],
    &[1, 1, 1, 1, 1, 1, 0, 1, 0],
    &[0, 1, 0, 0, 1, 1, 0, 1, 0],
    &[0, 1, 0, 1, 1, 1, 0, 0, 0],
    &[1, 1, 0, 1, 1, 1, 1, 1, 0],
    &[0, 1, 0, 1, 1, 1, 1, 1, 1],
    &[0, 0, 0, 1, 1, 1, 0, 1, 0],
    &[0, 1, 0, 1, 1, 0, 0, 1, 0],
    &[0, 1, 0, 1, 1, 1, 0, 1, 1],
    &[0, 1, 1, 1, 1, 1, 0, 1, 0],
    &[0, 1, 0, 1, 1, 1, 1, 1, 0],
    &[1, 1, 0, 1, 1, 1, 0, 1, 0],
];

pub fn fix_rule(rule: &[u8]) -> [u8; 9] {
    let mut fixed_rule = [0; 9];
    for (i, &value) in rule.iter().enumerate() {
        fixed_rule[i] = value;
    }
    // If the top is 1, the top left and top right become 2
    if fixed_rule[1] == 0 {
        fixed_rule[0] = 2;
        fixed_rule[2] = 2;
    }
    // If the right is 1, the top right and bottom right become 2
    if fixed_rule[5] == 0 {
        fixed_rule[2] = 2;
        fixed_rule[8] = 2;
    }
    // If the bottom is 1, the bottom left and bottom right become 2
    if fixed_rule[7] == 0 {
        fixed_rule[6] = 2;
        fixed_rule[8] = 2;
    }
    // If the left is 1, the top left and bottom left become 2
    if fixed_rule[3] == 0 {
        fixed_rule[0] = 2;
        fixed_rule[6] = 2;
    }
    fixed_rule
}

pub fn rule_complexity(rule: &[u8]) -> usize {
    rule.iter().filter(|&&x| x > 0).count()
}

/// Returns the index of the matching rule with most complexity.
pub fn match_adjacency_rule(neighborhood: &[bool; 9]) -> Option<usize> {
    let mut max_complexity = 0;
    let mut best_match = None;
    for (i, rule) in ADJACENCY_RULES.iter().enumerate() {
        let fixed_rule = fix_rule(rule);
        if neighborhood
            .iter()
            .zip(fixed_rule)
            .all(|(&neighbor, rule)| {
                if rule == 2 {
                    true
                } else {
                    neighbor == (rule == 1)
                }
            })
        {
            let complexity = rule_complexity(rule);
            if complexity >= max_complexity {
                max_complexity = complexity;
                best_match = Some(i);
            }
        }
    }
    best_match
}
