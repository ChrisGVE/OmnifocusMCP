//! Unit tests for the lenient scalar newtypes used by tool parameters.
//!
//! Some MCP clients serialize every tool argument as a JSON string (upstream
//! issues #8 and #11), so `estimatedMinutes: "30"` or `sequential: "true"` must
//! be accepted while still advertising the native JSON type in the schema.

use omnifocus_mcp::lenient_scalars::{LenientBool, LenientF64, LenientI32};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn parse<T: DeserializeOwned>(input: Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(input)
}

fn rejection_message<T: DeserializeOwned + std::fmt::Debug>(input: Value) -> String {
    match parse::<T>(input.clone()) {
        Ok(value) => panic!("{input} should be rejected, got {value:?}"),
        Err(error) => error.to_string(),
    }
}

// ---- LenientI32 -------------------------------------------------------------

#[test]
fn i32_accepts_native_integer() {
    assert_eq!(parse::<LenientI32>(json!(30)).unwrap(), LenientI32(30));
}

#[test]
fn i32_accepts_native_negative_integer() {
    assert_eq!(parse::<LenientI32>(json!(-5)).unwrap(), LenientI32(-5));
}

#[test]
fn i32_accepts_numeric_string() {
    assert_eq!(parse::<LenientI32>(json!("30")).unwrap(), LenientI32(30));
}

#[test]
fn i32_accepts_whitespace_padded_numeric_string() {
    assert_eq!(parse::<LenientI32>(json!(" 45 ")).unwrap(), LenientI32(45));
}

#[test]
fn i32_accepts_integral_float() {
    assert_eq!(parse::<LenientI32>(json!(30.0)).unwrap(), LenientI32(30));
}

#[test]
fn i32_rejects_fractional_float() {
    let message = rejection_message::<LenientI32>(json!(30.5));
    assert!(message.contains("expected an integer"), "{message}");
}

#[test]
fn i32_rejects_non_numeric_string() {
    let message = rejection_message::<LenientI32>(json!("abc"));
    assert!(message.contains("expected an integer"), "{message}");
}

#[test]
fn i32_rejects_empty_string() {
    let message = rejection_message::<LenientI32>(json!(""));
    assert!(message.contains("expected an integer"), "{message}");
}

#[test]
fn i32_rejects_whitespace_only_string() {
    rejection_message::<LenientI32>(json!("   "));
}

#[test]
fn i32_rejects_decimal_string() {
    // Strings are kept to plain whole numbers; "30.0" is not one.
    rejection_message::<LenientI32>(json!("30.0"));
}

#[test]
fn i32_rejects_integer_above_range() {
    let message = rejection_message::<LenientI32>(json!(2_147_483_648_i64));
    assert!(message.contains("expected an integer"), "{message}");
}

#[test]
fn i32_rejects_integer_below_range() {
    rejection_message::<LenientI32>(json!(-2_147_483_649_i64));
}

#[test]
fn i32_rejects_out_of_range_string() {
    rejection_message::<LenientI32>(json!("2147483648"));
}

#[test]
fn i32_rejects_out_of_range_integral_float() {
    rejection_message::<LenientI32>(json!(3.0e10));
}

#[test]
fn i32_rejects_boolean() {
    rejection_message::<LenientI32>(json!(true));
}

#[test]
fn i32_serializes_as_plain_integer() {
    assert_eq!(serde_json::to_value(LenientI32(30)).unwrap(), json!(30));
}

#[test]
fn i32_converts_into_native_value() {
    assert_eq!(i32::from(LenientI32(30)), 30);
}

// ---- LenientF64 -------------------------------------------------------------

#[test]
fn f64_accepts_native_float() {
    assert_eq!(parse::<LenientF64>(json!(1.5)).unwrap(), LenientF64(1.5));
}

#[test]
fn f64_accepts_native_negative_integer() {
    assert_eq!(
        parse::<LenientF64>(json!(-3600)).unwrap(),
        LenientF64(-3600.0)
    );
}

#[test]
fn f64_accepts_native_positive_integer() {
    assert_eq!(parse::<LenientF64>(json!(60)).unwrap(), LenientF64(60.0));
}

#[test]
fn f64_accepts_numeric_string() {
    assert_eq!(
        parse::<LenientF64>(json!(" -3600.5 ")).unwrap(),
        LenientF64(-3600.5)
    );
}

#[test]
fn f64_rejects_empty_string() {
    let message = rejection_message::<LenientF64>(json!(""));
    assert!(message.contains("expected a number"), "{message}");
}

#[test]
fn f64_rejects_non_numeric_string() {
    rejection_message::<LenientF64>(json!("abc"));
}

#[test]
fn f64_rejects_non_finite_string() {
    rejection_message::<LenientF64>(json!("NaN"));
    rejection_message::<LenientF64>(json!("inf"));
}

#[test]
fn f64_serializes_as_plain_number() {
    assert_eq!(serde_json::to_value(LenientF64(1.5)).unwrap(), json!(1.5));
}

#[test]
fn f64_converts_into_native_value() {
    assert_eq!(f64::from(LenientF64(1.5)), 1.5);
}

// ---- LenientBool ------------------------------------------------------------

#[test]
fn bool_accepts_native_true() {
    assert_eq!(
        parse::<LenientBool>(json!(true)).unwrap(),
        LenientBool(true)
    );
}

#[test]
fn bool_accepts_native_false() {
    assert_eq!(
        parse::<LenientBool>(json!(false)).unwrap(),
        LenientBool(false)
    );
}

#[test]
fn bool_accepts_true_string() {
    assert_eq!(
        parse::<LenientBool>(json!("true")).unwrap(),
        LenientBool(true)
    );
}

#[test]
fn bool_accepts_uppercase_false_string() {
    assert_eq!(
        parse::<LenientBool>(json!("FALSE")).unwrap(),
        LenientBool(false)
    );
}

#[test]
fn bool_accepts_whitespace_padded_mixed_case_string() {
    assert_eq!(
        parse::<LenientBool>(json!(" True ")).unwrap(),
        LenientBool(true)
    );
}

#[test]
fn bool_rejects_yes_string() {
    let message = rejection_message::<LenientBool>(json!("yes"));
    assert!(message.contains("expected a boolean"), "{message}");
}

#[test]
fn bool_rejects_one_string() {
    rejection_message::<LenientBool>(json!("1"));
}

#[test]
fn bool_rejects_empty_string() {
    rejection_message::<LenientBool>(json!(""));
}

#[test]
fn bool_rejects_native_integer() {
    rejection_message::<LenientBool>(json!(1));
}

#[test]
fn bool_serializes_as_plain_boolean() {
    assert_eq!(
        serde_json::to_value(LenientBool(true)).unwrap(),
        json!(true)
    );
}

#[test]
fn bool_converts_into_native_value() {
    assert!(bool::from(LenientBool(true)));
}

// ---- Option wrapping and schema delegation ----------------------------------

#[test]
fn option_of_lenient_maps_null_to_none() {
    assert_eq!(parse::<Option<LenientI32>>(Value::Null).unwrap(), None);
    assert_eq!(parse::<Option<LenientF64>>(Value::Null).unwrap(), None);
    assert_eq!(parse::<Option<LenientBool>>(Value::Null).unwrap(), None);
}

#[test]
fn schemas_are_identical_to_native_types() {
    use schemars::schema_for;
    assert_eq!(schema_for!(LenientI32), schema_for!(i32));
    assert_eq!(schema_for!(LenientF64), schema_for!(f64));
    assert_eq!(schema_for!(LenientBool), schema_for!(bool));
    assert_eq!(schema_for!(Option<LenientI32>), schema_for!(Option<i32>));
    assert_eq!(schema_for!(Option<LenientF64>), schema_for!(Option<f64>));
    assert_eq!(schema_for!(Option<LenientBool>), schema_for!(Option<bool>));
}
