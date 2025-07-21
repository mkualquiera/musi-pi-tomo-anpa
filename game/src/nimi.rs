use std::collections::HashMap;

use log::info;

const MAP: &[(&str, &str)] = &[
    ("a", "󱤀"),
    ("akesi", "󱤁"),
    ("ala", "󱤂"),
    ("alasa", "󱤃"),
    ("ale", "󱤄"),
    ("anpa", "󱤅"),
    ("ante", "󱤆"),
    ("anu", "󱤇"),
    ("awen", "󱤈"),
    ("e", "󱤉"),
    ("en", "󱤊"),
    ("esun", "󱤋"),
    ("ijo", "󱤌"),
    ("ike", "󱤍"),
    ("ilo", "󱤎"),
    ("insa", "󱤏"),
    ("a", "󱤀"),
    ("akesi", "󱤁"),
    ("ala", "󱤂"),
    ("alasa", "󱤃"),
    ("ale", "󱤄"),
    ("anpa", "󱤅"),
    ("ante", "󱤆"),
    ("anu", "󱤇"),
    ("awen", "󱤈"),
    ("e", "󱤉"),
    ("en", "󱤊"),
    ("esun", "󱤋"),
    ("ijo", "󱤌"),
    ("ike", "󱤍"),
    ("ilo", "󱤎"),
    ("insa", "󱤏"),
    ("jaki", "󱤐"),
    ("jan", "󱤑"),
    ("jelo", "󱤒"),
    ("jo", "󱤓"),
    ("kala", "󱤔"),
    ("kalama", "󱤕"),
    ("kama", "󱤖"),
    ("kasi", "󱤗"),
    ("ken", "󱤘"),
    ("kepeken", "󱤙"),
    ("kili", "󱤚"),
    ("kiwen", "󱤛"),
    ("ko", "󱤜"),
    ("kon", "󱤝"),
    ("kule", "󱤞"),
    ("kulupu", "󱤟"),
    ("kute", "󱤠"),
    ("la", "󱤡"),
    ("lape", "󱤢"),
    ("laso", "󱤣"),
    ("lawa", "󱤤"),
    ("len", "󱤥"),
    ("lete", "󱤦"),
    ("li", "󱤧"),
    ("lili", "󱤨"),
    ("linja", "󱤩"),
    ("lipu", "󱤪"),
    ("loje", "󱤫"),
    ("lon", "󱤬"),
    ("luka", "󱤭"),
    ("lukin", "󱤮"),
    ("lupa", "󱤯"),
    ("ma", "󱤰"),
    ("mama", "󱤱"),
    ("mani", "󱤲"),
    ("meli", "󱤳"),
    ("mi", "󱤴"),
    ("mije", "󱤵"),
    ("moku", "󱤶"),
    ("moli", "󱤷"),
    ("monsi", "󱤸"),
    ("mu", "󱤹"),
    ("mun", "󱤺"),
    ("musi", "󱤻"),
    ("mute", "󱤼"),
    ("nanpa", "󱤽"),
    ("nasa", "󱤾"),
    ("nasin", "󱤿"),
    ("nena", "󱥀"),
    ("ni", "󱥁"),
    ("nimi", "󱥂"),
    ("noka", "󱥃"),
    ("o", "󱥄"),
    ("olin", "󱥅"),
    ("ona", "󱥆"),
    ("open", "󱥇"),
    ("pakala", "󱥈"),
    ("pali", "󱥉"),
    ("palisa", "󱥊"),
    ("pan", "󱥋"),
    ("pana", "󱥌"),
    ("pi", "󱥍"),
    ("pilin", "󱥎"),
    ("pimeja", "󱥏"),
    ("pini", "󱥐"),
    ("pipi", "󱥑"),
    ("poka", "󱥒"),
    ("poki", "󱥓"),
    ("pona", "󱥔"),
    ("pu", "󱥕"),
    ("sama", "󱥖"),
    ("seli", "󱥗"),
    ("selo", "󱥘"),
    ("seme", "󱥙"),
    ("sewi", "󱥚"),
    ("sijelo", "󱥛"),
    ("sike", "󱥜"),
    ("sin", "󱥝"),
    ("sina", "󱥞"),
    ("sinpin", "󱥟"),
    ("sitelen", "󱥠"),
    ("sona", "󱥡"),
    ("soweli", "󱥢"),
    ("suli", "󱥣"),
    ("suno", "󱥤"),
    ("supa", "󱥥"),
    ("suwi", "󱥦"),
    ("tan", "󱥧"),
    ("taso", "󱥨"),
    ("tawa", "󱥩"),
    ("telo", "󱥪"),
    ("tenpo", "󱥫"),
    ("toki", "󱥬"),
    ("tomo", "󱥭"),
    ("tu", "󱥮"),
    ("unpa", "󱥯"),
    ("uta", "󱥰"),
    ("utala", "󱥱"),
    ("walo", "󱥲"),
    ("wan", "󱥳"),
    ("waso", "󱥴"),
    ("wawa", "󱥵"),
    ("weka", "󱥶"),
    ("wile", "󱥷"),
    ("namako", "󱥸"),
    ("kin", "󱥹"),
    ("oko", "󱥺"),
    ("kipisi", "󱥻"),
    ("leko", "󱥼"),
    ("monsuta", "󱥽"),
    ("tonsi", "󱥾"),
    ("jasima", "󱥿"),
    ("kijetesantakalu", "󱦀"),
    ("soko", "󱦁"),
    ("meso", "󱦂"),
    ("epiku", "󱦃"),
    ("kokosila", "󱦄"),
    ("lanpan", "󱦅"),
    ("n", "󱦆"),
    ("misikeke", "󱦇"),
    ("ku", "󱦈"),
    ("pake", "󱦠"),
    ("apeja", "󱦡"),
    ("majuna", "󱦢"),
    ("powe", "󱦣"),
    ("[", "󱦐"),
    ("]", "󱦑"),
    ("_", "󱦒"),
    ("pi_", "󱦓"),
    ("'", "󱦔"),
];

pub fn convert_latin_to_ucsur(text: &str) -> String {
    // Create HashMap from the const array and sort by key length (descending)
    let mut table: Vec<(&str, &str)> = MAP.iter().copied().collect();
    table.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let table_map: HashMap<&str, &str> = table.into_iter().collect();

    let mut current_text = text;
    let mut output_text = String::new();

    while !current_text.is_empty() {
        let mut matched = false;

        // Try to match the longest possible word first
        for &(latin, ucsur) in MAP.iter() {
            if current_text.starts_with(latin) {
                // Check if this is a complete word boundary
                let after_match = &current_text[latin.len()..];
                if after_match.is_empty() || !after_match.chars().next().unwrap().is_alphabetic() {
                    output_text.push_str(ucsur);
                    current_text = after_match;
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            // Take the first character and add it to output
            if let Some(ch) = current_text.chars().next() {
                if ch.is_whitespace() {
                    // Skip whitespace (as per your Python version)
                } else {
                    output_text.push(ch);
                }
                current_text = &current_text[ch.len_utf8()..];
            } else {
                break;
            }
        }
    }

    output_text
}

fn factorize_mixed_radix(number: i32) -> String {
    let basis = vec![100, 20, 5, 2, 1];

    if number == 0 {
        return "0".to_string();
    }

    let mut terms: Vec<Vec<i32>> = Vec::new();
    let mut remaining = number;

    // Process each basis element in descending order
    for &base in &basis {
        if base == 1 {
            // Handle 1s at the end
            if remaining > 0 {
                for _ in 0..remaining {
                    terms.push(vec![1]);
                }
                remaining = 0;
            }
        } else {
            // Build the largest possible product starting with this base
            while remaining >= base {
                if let Some(product_term) = build_greedy_product(remaining, base, &basis) {
                    let product_value = calculate_product(&product_term);
                    terms.push(product_term);
                    remaining -= product_value;
                } else {
                    break;
                }
            }
        }
    }

    // Format the result
    format_result(terms)
}

fn build_greedy_product(limit: i32, start_base: i32, basis: &[i32]) -> Option<Vec<i32>> {
    if start_base > limit {
        return None;
    }

    let mut factors = vec![start_base];
    let mut current_product = start_base;

    // Keep trying to multiply by basis elements in descending order
    loop {
        let mut extended = false;

        for &base in basis {
            if base > 1 && current_product <= limit / base {
                factors.push(base);
                current_product *= base;
                extended = true;
                break; // Start over with the largest possible base
            }
        }

        if !extended {
            break;
        }
    }

    Some(factors)
}

fn calculate_product(factors: &[i32]) -> i32 {
    factors.iter().product()
}

fn format_result(terms: Vec<Vec<i32>>) -> String {
    let parts: Vec<String> = terms
        .into_iter()
        .map(|term| {
            term.into_iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join("*")
        })
        .collect();

    parts.join("+")
}

pub fn number_to_toki_pona(number: u32) -> String {
    let partial = factorize_mixed_radix(number as i32);
    // replace 100 with ale, 20 with mute, 5 with luka, 2 with tu, and 1 with wan
    // then * with a space
    // and + with "en"
    let mut result = partial
        .replace("100", "ale")
        .replace("20", "mute")
        .replace("5", "luka")
        .replace("2", "tu")
        .replace("1", "wan")
        .replace("0", "ala")
        .replace("*", " ")
        .replace("+", " en ");

    // remove any trailing spaces
    result = result.trim().to_string();
    // if the result is empty, return "ala"
    if result.is_empty() {
        result = "ala".to_string();
    }

    result
}
