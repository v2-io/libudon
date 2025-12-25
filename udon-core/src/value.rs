//! Attribute value types with syntactic typing.
//!
//! UDON uses syntactic typing - the syntax determines the type,
//! not value sniffing. These types are stable and hand-written.

/// Attribute value with syntactic type.
///
/// The lifetime `'a` refers to the source buffer - values are
/// zero-copy slices into the original input.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// Nil value: `null`, `nil`, or `~`
    Nil,

    /// Boolean: `true` or `false` (lowercase only)
    Bool(bool),

    /// Integer: `42`, `0xFF`, `0o755`, `0b1010`, etc.
    Integer(i64),

    /// Float: `3.14`, `1.5e-3`, etc.
    Float(f64),

    /// Rational: `1/3r`, `22/7r`
    Rational { numerator: i64, denominator: i64 },

    /// Complex: `3+4i`, `5i`
    Complex { real: f64, imag: f64 },

    /// String (unquoted bare string)
    String(&'a [u8]),

    /// Quoted string (needs unescaping)
    QuotedString(&'a [u8]),

    /// List: `[a b c]`
    List(Vec<Value<'a>>),
}

impl<'a> Value<'a> {
    /// Check if this is a nil value.
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Try to get as boolean.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as integer.
    #[inline]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as string bytes.
    #[inline]
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::String(s) | Value::QuotedString(s) => Some(s),
            _ => None,
        }
    }

    /// Parse a byte slice into a typed Value.
    ///
    /// Uses syntactic typing per SPEC.md:
    /// - `true`/`false` → Bool
    /// - `null`/`nil`/`~` → Nil
    /// - Integer patterns (42, 0xFF, 0o755, 0b1010) → Integer
    /// - Float patterns (3.14, 1.5e-3) → Float
    /// - Rational patterns (1/3r) → Rational
    /// - Complex patterns (3+4i, 5i) → Complex
    /// - Everything else → String
    pub fn parse(bytes: &'a [u8]) -> Value<'a> {
        if bytes.is_empty() {
            return Value::String(bytes);
        }

        // Check for nil values
        if bytes == b"null" || bytes == b"nil" || bytes == b"~" {
            return Value::Nil;
        }

        // Check for boolean values (lowercase only per SPEC)
        if bytes == b"true" {
            return Value::Bool(true);
        }
        if bytes == b"false" {
            return Value::Bool(false);
        }

        // Try parsing as a number
        if let Some(value) = Self::try_parse_number(bytes) {
            return value;
        }

        // Fallback to string
        Value::String(bytes)
    }

    /// Try to parse bytes as a numeric value.
    /// Returns None if not a valid number format.
    fn try_parse_number(bytes: &'a [u8]) -> Option<Value<'a>> {
        if bytes.is_empty() {
            return None;
        }

        // Check for complex number first (ends with 'i')
        if bytes.last() == Some(&b'i') {
            return Self::try_parse_complex(bytes);
        }

        // Check for rational (ends with 'r' after a slash)
        if bytes.last() == Some(&b'r') && bytes.contains(&b'/') {
            return Self::try_parse_rational(bytes);
        }

        let (negative, rest) = if bytes.first() == Some(&b'-') {
            (true, &bytes[1..])
        } else {
            (false, bytes)
        };

        if rest.is_empty() {
            return None;
        }

        // Check for base prefixes
        if rest.len() >= 2 && rest[0] == b'0' {
            match rest[1] {
                b'x' | b'X' => return Self::try_parse_hex(negative, &rest[2..]),
                b'o' | b'O' => return Self::try_parse_octal(negative, &rest[2..]),
                b'b' | b'B' => return Self::try_parse_binary(negative, &rest[2..]),
                b'd' | b'D' => return Self::try_parse_decimal(negative, &rest[2..]),
                _ => {}
            }
        }

        // Check if it contains a decimal point or exponent (float)
        if rest.contains(&b'.') || rest.contains(&b'e') || rest.contains(&b'E') {
            return Self::try_parse_float(negative, rest);
        }

        // Try as decimal integer
        Self::try_parse_decimal(negative, rest)
    }

    fn try_parse_decimal(negative: bool, bytes: &[u8]) -> Option<Value<'a>> {
        if bytes.is_empty() {
            return None;
        }

        let mut result: i64 = 0;
        for &b in bytes {
            match b {
                b'0'..=b'9' => {
                    result = result.checked_mul(10)?.checked_add((b - b'0') as i64)?;
                }
                b'_' => continue, // Underscore separator allowed
                _ => return None,
            }
        }

        if negative {
            result = result.checked_neg()?;
        }

        Some(Value::Integer(result))
    }

    fn try_parse_hex(negative: bool, bytes: &[u8]) -> Option<Value<'a>> {
        if bytes.is_empty() {
            return None;
        }

        let mut result: i64 = 0;
        for &b in bytes {
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                b'_' => continue,
                _ => return None,
            };
            result = result.checked_mul(16)?.checked_add(digit as i64)?;
        }

        if negative {
            result = result.checked_neg()?;
        }

        Some(Value::Integer(result))
    }

    fn try_parse_octal(negative: bool, bytes: &[u8]) -> Option<Value<'a>> {
        if bytes.is_empty() {
            return None;
        }

        let mut result: i64 = 0;
        for &b in bytes {
            match b {
                b'0'..=b'7' => {
                    result = result.checked_mul(8)?.checked_add((b - b'0') as i64)?;
                }
                b'_' => continue,
                _ => return None,
            }
        }

        if negative {
            result = result.checked_neg()?;
        }

        Some(Value::Integer(result))
    }

    fn try_parse_binary(negative: bool, bytes: &[u8]) -> Option<Value<'a>> {
        if bytes.is_empty() {
            return None;
        }

        let mut result: i64 = 0;
        for &b in bytes {
            match b {
                b'0' | b'1' => {
                    result = result.checked_mul(2)?.checked_add((b - b'0') as i64)?;
                }
                b'_' => continue,
                _ => return None,
            }
        }

        if negative {
            result = result.checked_neg()?;
        }

        Some(Value::Integer(result))
    }

    fn try_parse_float(negative: bool, bytes: &[u8]) -> Option<Value<'a>> {
        // Remove underscores and convert to string for parsing
        let s: String = bytes
            .iter()
            .filter(|&&b| b != b'_')
            .map(|&b| b as char)
            .collect();

        let mut value: f64 = s.parse().ok()?;
        if negative {
            value = -value;
        }

        Some(Value::Float(value))
    }

    fn try_parse_rational(bytes: &'a [u8]) -> Option<Value<'a>> {
        // Format: [numerator]/[denominator]r
        let without_r = &bytes[..bytes.len() - 1]; // Remove trailing 'r'
        let slash_pos = without_r.iter().position(|&b| b == b'/')?;

        let num_bytes = &without_r[..slash_pos];
        let denom_bytes = &without_r[slash_pos + 1..];

        // Parse numerator (may be negative)
        let (negative, num_rest) = if num_bytes.first() == Some(&b'-') {
            (true, &num_bytes[1..])
        } else {
            (false, num_bytes)
        };

        let numerator = Self::parse_digits(num_rest)?;
        let numerator = if negative { -numerator } else { numerator };

        let denominator = Self::parse_digits(denom_bytes)?;
        if denominator == 0 {
            return None; // Division by zero
        }

        Some(Value::Rational { numerator, denominator })
    }

    fn try_parse_complex(bytes: &'a [u8]) -> Option<Value<'a>> {
        // Format: [real][+/-][imag]i or just [imag]i
        let without_i = &bytes[..bytes.len() - 1];

        if without_i.is_empty() {
            return None;
        }

        // Find the last + or - that separates real and imaginary parts
        // (but not at the start, which would be a negative number)
        let mut split_pos = None;
        for (i, &b) in without_i.iter().enumerate().rev() {
            if (b == b'+' || b == b'-') && i > 0 {
                // Make sure it's not part of an exponent
                if i > 0 && (without_i[i - 1] == b'e' || without_i[i - 1] == b'E') {
                    continue;
                }
                split_pos = Some(i);
                break;
            }
        }

        if let Some(pos) = split_pos {
            // real + imag i
            let real_bytes = &without_i[..pos];
            let imag_bytes = &without_i[pos..];

            let real = Self::parse_float_simple(real_bytes)?;
            let imag = Self::parse_float_simple(imag_bytes)?;

            Some(Value::Complex { real, imag })
        } else {
            // Pure imaginary: just [imag]i
            let imag = Self::parse_float_simple(without_i)?;
            Some(Value::Complex { real: 0.0, imag })
        }
    }

    fn parse_digits(bytes: &[u8]) -> Option<i64> {
        if bytes.is_empty() {
            return None;
        }

        let mut result: i64 = 0;
        for &b in bytes {
            match b {
                b'0'..=b'9' => {
                    result = result.checked_mul(10)?.checked_add((b - b'0') as i64)?;
                }
                b'_' => continue,
                _ => return None,
            }
        }
        Some(result)
    }

    fn parse_float_simple(bytes: &[u8]) -> Option<f64> {
        let s: String = bytes
            .iter()
            .filter(|&&b| b != b'_')
            .map(|&b| b as char)
            .collect();

        s.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nil_values() {
        assert_eq!(Value::parse(b"null"), Value::Nil);
        assert_eq!(Value::parse(b"nil"), Value::Nil);
        assert_eq!(Value::parse(b"~"), Value::Nil);
    }

    #[test]
    fn test_boolean_values() {
        assert_eq!(Value::parse(b"true"), Value::Bool(true));
        assert_eq!(Value::parse(b"false"), Value::Bool(false));
        // Case sensitive - these should be strings
        assert_eq!(Value::parse(b"TRUE"), Value::String(b"TRUE"));
        assert_eq!(Value::parse(b"True"), Value::String(b"True"));
    }

    #[test]
    fn test_integer_values() {
        assert_eq!(Value::parse(b"42"), Value::Integer(42));
        assert_eq!(Value::parse(b"0"), Value::Integer(0));
        assert_eq!(Value::parse(b"-42"), Value::Integer(-42));
        assert_eq!(Value::parse(b"1_000_000"), Value::Integer(1_000_000));
    }

    #[test]
    fn test_hex_values() {
        assert_eq!(Value::parse(b"0xFF"), Value::Integer(255));
        assert_eq!(Value::parse(b"0x10"), Value::Integer(16));
        assert_eq!(Value::parse(b"0xDEAD_BEEF"), Value::Integer(0xDEADBEEF));
    }

    #[test]
    fn test_octal_values() {
        assert_eq!(Value::parse(b"0o755"), Value::Integer(493)); // 755 octal
        assert_eq!(Value::parse(b"0o10"), Value::Integer(8));
    }

    #[test]
    fn test_binary_values() {
        assert_eq!(Value::parse(b"0b1010"), Value::Integer(10));
        assert_eq!(Value::parse(b"0b1111_0000"), Value::Integer(240));
    }

    #[test]
    fn test_float_values() {
        assert_eq!(Value::parse(b"3.14"), Value::Float(3.14));
        assert_eq!(Value::parse(b"1.5e-3"), Value::Float(0.0015));
        assert_eq!(Value::parse(b"-2.5"), Value::Float(-2.5));
    }

    #[test]
    fn test_rational_values() {
        assert_eq!(Value::parse(b"1/3r"), Value::Rational { numerator: 1, denominator: 3 });
        assert_eq!(Value::parse(b"22/7r"), Value::Rational { numerator: 22, denominator: 7 });
        assert_eq!(Value::parse(b"-1/2r"), Value::Rational { numerator: -1, denominator: 2 });
    }

    #[test]
    fn test_complex_values() {
        assert_eq!(Value::parse(b"5i"), Value::Complex { real: 0.0, imag: 5.0 });
        assert_eq!(Value::parse(b"3+4i"), Value::Complex { real: 3.0, imag: 4.0 });
        assert_eq!(Value::parse(b"3-4i"), Value::Complex { real: 3.0, imag: -4.0 });
    }

    #[test]
    fn test_string_fallback() {
        assert_eq!(Value::parse(b"hello"), Value::String(b"hello"));
        assert_eq!(Value::parse(b"hello-world"), Value::String(b"hello-world"));
        assert_eq!(Value::parse(b"not-a-number"), Value::String(b"not-a-number"));
    }

    #[test]
    fn test_edge_cases() {
        // Empty string
        assert_eq!(Value::parse(b""), Value::String(b""));
        // Leading zeros (should be decimal, not octal)
        assert_eq!(Value::parse(b"0755"), Value::Integer(755));
        // Explicit decimal
        assert_eq!(Value::parse(b"0d42"), Value::Integer(42));
    }
}
