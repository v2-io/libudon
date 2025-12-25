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
}
