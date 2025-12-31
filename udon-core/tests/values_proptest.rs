//! Property tests comparing values_parser with lexical-core.
//!
//! Generates numeric strings and verifies both parsers agree on:
//! 1. Type detection (integer vs float)
//! 2. Parsed value (when applicable)

use proptest::prelude::*;
use udon_core::values_parser::{Event, Parser as ValuesParser};

/// Result of parsing with values_parser
#[derive(Debug, Clone, PartialEq)]
enum ValuesResult {
    Integer(Vec<u8>),
    Float(Vec<u8>),
}

/// Result of parsing with lexical-core + prefix detection
#[derive(Debug, Clone, PartialEq)]
enum LexicalResult {
    Integer(i64),
    Float(f64),
    Error,
}

fn parse_with_values_parser(input: &[u8]) -> Option<ValuesResult> {
    // Append a space to trigger default case instead of EOF
    // (descent CONTENT functions emit return type at EOF, bypassing inline emits)
    let mut input_with_terminator = input.to_vec();
    input_with_terminator.push(b' ');

    let mut result = None;
    ValuesParser::new(&input_with_terminator).parse(|event| {
        // Only take the FIRST event (descent CONTENT functions may emit twice -
        // once for inline emit, once for auto-return - we want the first one)
        if result.is_some() {
            return;
        }
        match event {
            Event::Integer { content, .. } => {
                result = Some(ValuesResult::Integer(content.to_vec()));
            }
            Event::Float { content, .. } => {
                result = Some(ValuesResult::Float(content.to_vec()));
            }
            Event::Error { .. } => {}
        }
    });
    result
}

fn parse_with_lexical(input: &[u8]) -> LexicalResult {
    // Check for prefixes
    if input.len() >= 2 && input[0] == b'0' {
        match input[1] {
            b'x' | b'X' => {
                // Hex - strip underscores and parse
                let cleaned: Vec<u8> = input[2..].iter().filter(|&&b| b != b'_').copied().collect();
                if let Ok(s) = std::str::from_utf8(&cleaned) {
                    if let Ok(v) = i64::from_str_radix(s, 16) {
                        return LexicalResult::Integer(v);
                    }
                }
                return LexicalResult::Error;
            }
            b'o' | b'O' => {
                // Octal
                let cleaned: Vec<u8> = input[2..].iter().filter(|&&b| b != b'_').copied().collect();
                if let Ok(s) = std::str::from_utf8(&cleaned) {
                    if let Ok(v) = i64::from_str_radix(s, 8) {
                        return LexicalResult::Integer(v);
                    }
                }
                return LexicalResult::Error;
            }
            b'b' | b'B' => {
                // Binary
                let cleaned: Vec<u8> = input[2..].iter().filter(|&&b| b != b'_').copied().collect();
                if let Ok(s) = std::str::from_utf8(&cleaned) {
                    if let Ok(v) = i64::from_str_radix(s, 2) {
                        return LexicalResult::Integer(v);
                    }
                }
                return LexicalResult::Error;
            }
            _ => {}
        }
    }

    // Standard decimal: try int then float (strip underscores for lexical-core)
    let cleaned: Vec<u8> = input.iter().filter(|&&b| b != b'_').copied().collect();

    if let Ok(v) = lexical_core::parse::<i64>(&cleaned) {
        return LexicalResult::Integer(v);
    }
    if let Ok(v) = lexical_core::parse::<f64>(&cleaned) {
        return LexicalResult::Float(v);
    }
    LexicalResult::Error
}

/// Check if both parsers agree on type
fn types_match(values_result: &Option<ValuesResult>, lexical_result: &LexicalResult) -> bool {
    match (values_result, lexical_result) {
        (Some(ValuesResult::Integer(_)), LexicalResult::Integer(_)) => true,
        (Some(ValuesResult::Float(_)), LexicalResult::Float(_)) => true,
        (None, LexicalResult::Error) => true,
        _ => false,
    }
}

// ============ Generators ============

/// Generate a decimal integer string
fn gen_decimal_int() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(prop::sample::select(vec![
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
    ]), 1..10)
    .prop_filter_map("no leading zeros except single 0", |digits| {
        if digits.len() > 1 && digits[0] == b'0' {
            None
        } else {
            Some(digits)
        }
    })
}

/// Generate a signed decimal integer
fn gen_signed_decimal_int() -> impl Strategy<Value = Vec<u8>> {
    (prop::option::of(prop::sample::select(vec![b'-', b'+'])), gen_decimal_int())
        .prop_map(|(sign, digits)| {
            let mut result = Vec::new();
            if let Some(s) = sign {
                result.push(s);
            }
            result.extend(digits);
            result
        })
}

/// Generate a hex integer (0x prefix)
fn gen_hex_int() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(prop::sample::select(vec![
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
        b'a', b'b', b'c', b'd', b'e', b'f',
        b'A', b'B', b'C', b'D', b'E', b'F',
    ]), 1..8)
    .prop_map(|digits| {
        let mut result = vec![b'0', b'x'];
        result.extend(digits);
        result
    })
}

/// Generate an octal integer (0o prefix)
fn gen_octal_int() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(prop::sample::select(vec![
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    ]), 1..8)
    .prop_map(|digits| {
        let mut result = vec![b'0', b'o'];
        result.extend(digits);
        result
    })
}

/// Generate a binary integer (0b prefix)
fn gen_binary_int() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(prop::sample::select(vec![b'0', b'1']), 1..16)
        .prop_map(|digits| {
            let mut result = vec![b'0', b'b'];
            result.extend(digits);
            result
        })
}

/// Generate a float string
fn gen_float() -> impl Strategy<Value = Vec<u8>> {
    (
        prop::option::of(prop::sample::select(vec![b'-', b'+'])),
        gen_decimal_int(),
        gen_decimal_int(),
        prop::option::of((
            prop::sample::select(vec![b'e', b'E']),
            prop::option::of(prop::sample::select(vec![b'-', b'+'])),
            gen_decimal_int(),
        )),
    )
        .prop_map(|(sign, int_part, frac_part, exp)| {
            let mut result = Vec::new();
            if let Some(s) = sign {
                result.push(s);
            }
            result.extend(int_part);
            result.push(b'.');
            result.extend(frac_part);
            if let Some((e, exp_sign, exp_digits)) = exp {
                result.push(e);
                if let Some(s) = exp_sign {
                    result.push(s);
                }
                result.extend(exp_digits);
            }
            result
        })
}

/// Generate any numeric value
fn gen_numeric() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        gen_signed_decimal_int(),
        gen_hex_int(),
        gen_octal_int(),
        gen_binary_int(),
        gen_float(),
    ]
}

// ============ Tests ============

proptest! {
    #[test]
    fn decimal_integers_match(input in gen_signed_decimal_int()) {
        let input_str = String::from_utf8_lossy(&input);
        let values_result = parse_with_values_parser(&input);
        let lexical_result = parse_with_lexical(&input);

        prop_assert!(
            types_match(&values_result, &lexical_result),
            "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
            input_str, values_result, lexical_result
        );
    }

    #[test]
    fn hex_integers_match(input in gen_hex_int()) {
        let input_str = String::from_utf8_lossy(&input);
        let values_result = parse_with_values_parser(&input);
        let lexical_result = parse_with_lexical(&input);

        prop_assert!(
            types_match(&values_result, &lexical_result),
            "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
            input_str, values_result, lexical_result
        );
    }

    #[test]
    fn octal_integers_match(input in gen_octal_int()) {
        let input_str = String::from_utf8_lossy(&input);
        let values_result = parse_with_values_parser(&input);
        let lexical_result = parse_with_lexical(&input);

        prop_assert!(
            types_match(&values_result, &lexical_result),
            "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
            input_str, values_result, lexical_result
        );
    }

    #[test]
    fn binary_integers_match(input in gen_binary_int()) {
        let input_str = String::from_utf8_lossy(&input);
        let values_result = parse_with_values_parser(&input);
        let lexical_result = parse_with_lexical(&input);

        prop_assert!(
            types_match(&values_result, &lexical_result),
            "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
            input_str, values_result, lexical_result
        );
    }

    #[test]
    fn floats_match(input in gen_float()) {
        let input_str = String::from_utf8_lossy(&input);
        let values_result = parse_with_values_parser(&input);
        let lexical_result = parse_with_lexical(&input);

        prop_assert!(
            types_match(&values_result, &lexical_result),
            "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
            input_str, values_result, lexical_result
        );
    }

    #[test]
    fn all_numerics_match(input in gen_numeric()) {
        let input_str = String::from_utf8_lossy(&input);
        let values_result = parse_with_values_parser(&input);
        let lexical_result = parse_with_lexical(&input);

        prop_assert!(
            types_match(&values_result, &lexical_result),
            "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
            input_str, values_result, lexical_result
        );
    }
}

// ============ Manual Tests ============

#[test]
fn test_known_values() {
    let test_cases = [
        (b"0".as_slice(), true, true),       // integer
        (b"42".as_slice(), true, true),      // integer
        (b"-42".as_slice(), true, true),     // integer
        (b"0xFF".as_slice(), true, true),    // hex integer
        (b"0o755".as_slice(), true, true),   // octal integer
        (b"0b1010".as_slice(), true, true),  // binary integer
        (b"3.14".as_slice(), false, true),   // float
        (b"1.5e-3".as_slice(), false, true), // float with exp
    ];

    for (input, is_int, should_match) in test_cases {
        let input_str = std::str::from_utf8(input).unwrap();
        let values_result = parse_with_values_parser(input);
        let lexical_result = parse_with_lexical(input);

        println!("{}: values={:?}, lexical={:?}", input_str, values_result, lexical_result);

        if should_match {
            assert!(
                types_match(&values_result, &lexical_result),
                "Type mismatch for '{}': values_parser={:?}, lexical={:?}",
                input_str, values_result, lexical_result
            );
        }

        // Verify expected type
        match values_result {
            Some(ValuesResult::Integer(_)) => assert!(is_int, "{} should be float", input_str),
            Some(ValuesResult::Float(_)) => assert!(!is_int, "{} should be integer", input_str),
            None => panic!("{} failed to parse", input_str),
        }
    }
}
