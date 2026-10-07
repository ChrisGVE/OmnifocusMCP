//! Numeric and boolean tool parameters that also accept their string encoding.
//!
//! Some MCP clients serialize every tool argument as a JSON string, so a field
//! declared as an integer arrives as `"30"` and a boolean as `"true"` (upstream
//! issues #8 and #11). rmcp hands the arguments to plain serde, which rejects
//! those strings. The newtypes here accept both the native JSON value and its
//! string form.
//!
//! The advertised tool schema must not change: clients should still be told to
//! send `integer`, `number`, or `boolean`. Each newtype therefore forwards its
//! `JsonSchema` to the native type, the same way schemars forwards its own
//! wrapper types, and serializes as the bare inner value.
//!
//! Fields use them as `Option<LenientI32>` (etc.). A missing field or a JSON
//! `null` becomes `None` through serde's ordinary `Option` handling.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::{Deserialize, Deserializer, Error as DeError, Unexpected, Visitor};
use serde::ser::{Serialize, Serializer};

/// An `i32` that also accepts a string holding a whole number (`"30"`, `" 45 "`).
/// A JSON float is accepted only when it is integral and in range (`30.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LenientI32(pub i32);

/// An `f64` that also accepts a string holding a finite number (`"-3600"`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LenientF64(pub f64);

/// A `bool` that also accepts the strings `"true"` / `"false"`, case-insensitive
/// and ignoring surrounding whitespace. Nothing else (`"yes"`, `"1"`) is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LenientBool(pub bool);

/// Makes `$wrapper` look exactly like `$native` from the outside: same JSON
/// schema, same serialized form, and a lossless conversion back to `$native`.
macro_rules! present_as_native {
    ($wrapper:ident => $native:ty) => {
        impl JsonSchema for $wrapper {
            fn inline_schema() -> bool {
                <$native as JsonSchema>::inline_schema()
            }

            fn schema_name() -> Cow<'static, str> {
                <$native as JsonSchema>::schema_name()
            }

            fn schema_id() -> Cow<'static, str> {
                <$native as JsonSchema>::schema_id()
            }

            fn json_schema(generator: &mut SchemaGenerator) -> Schema {
                <$native as JsonSchema>::json_schema(generator)
            }
        }

        impl Serialize for $wrapper {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(serializer)
            }
        }

        impl From<$wrapper> for $native {
            fn from(value: $wrapper) -> Self {
                value.0
            }
        }
    };
}

present_as_native!(LenientI32 => i32);
present_as_native!(LenientF64 => f64);
present_as_native!(LenientBool => bool);

struct LenientI32Visitor;

impl Visitor<'_> for LenientI32Visitor {
    type Value = LenientI32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an integer in the i32 range, or a string containing one (e.g. 30 or \"30\")")
    }

    fn visit_i64<E: DeError>(self, v: i64) -> Result<Self::Value, E> {
        i32::try_from(v)
            .map(LenientI32)
            .map_err(|_| E::invalid_value(Unexpected::Signed(v), &self))
    }

    fn visit_u64<E: DeError>(self, v: u64) -> Result<Self::Value, E> {
        i32::try_from(v)
            .map(LenientI32)
            .map_err(|_| E::invalid_value(Unexpected::Unsigned(v), &self))
    }

    fn visit_f64<E: DeError>(self, v: f64) -> Result<Self::Value, E> {
        let in_range = v >= f64::from(i32::MIN) && v <= f64::from(i32::MAX);
        if in_range && v.fract() == 0.0 {
            // Integral and within range, so the cast is exact.
            Ok(LenientI32(v as i32))
        } else {
            Err(E::invalid_value(Unexpected::Float(v), &self))
        }
    }

    fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
        v.trim()
            .parse::<i32>()
            .map(LenientI32)
            .map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
    }
}

impl<'de> Deserialize<'de> for LenientI32 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LenientI32Visitor)
    }
}

struct LenientF64Visitor;

impl Visitor<'_> for LenientF64Visitor {
    type Value = LenientF64;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number, or a string containing a finite number (e.g. -3600 or \"-3600\")")
    }

    fn visit_i64<E: DeError>(self, v: i64) -> Result<Self::Value, E> {
        Ok(LenientF64(v as f64))
    }

    fn visit_u64<E: DeError>(self, v: u64) -> Result<Self::Value, E> {
        Ok(LenientF64(v as f64))
    }

    fn visit_f64<E: DeError>(self, v: f64) -> Result<Self::Value, E> {
        Ok(LenientF64(v))
    }

    fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
        // `str::parse::<f64>` also accepts "NaN" and "inf"; neither is a usable offset.
        match v.trim().parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(LenientF64(number)),
            _ => Err(E::invalid_value(Unexpected::Str(v), &self)),
        }
    }
}

impl<'de> Deserialize<'de> for LenientF64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LenientF64Visitor)
    }
}

struct LenientBoolVisitor;

impl Visitor<'_> for LenientBoolVisitor {
    type Value = LenientBool;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a boolean, or the string \"true\" or \"false\" (case-insensitive)")
    }

    fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
        Ok(LenientBool(v))
    }

    fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
        let word = v.trim();
        if word.eq_ignore_ascii_case("true") {
            Ok(LenientBool(true))
        } else if word.eq_ignore_ascii_case("false") {
            Ok(LenientBool(false))
        } else {
            Err(E::invalid_value(Unexpected::Str(v), &self))
        }
    }
}

impl<'de> Deserialize<'de> for LenientBool {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LenientBoolVisitor)
    }
}
