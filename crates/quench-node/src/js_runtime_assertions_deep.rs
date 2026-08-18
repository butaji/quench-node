// Deep/loose equality primitives for `node:assert`. These mirror the core of
// Node's `lib/internal/assert/assertion_error.js` comparison semantics used by
// `strictEqual` (Object.is), `deepEqual` (loose), and `deepStrictEqual`.

/// Object.is() for the value kinds the reduced engine represents.
fn assert_object_is(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(actual), Value::Number(expected)) => {
            (actual.is_nan() && expected.is_nan())
                || (*actual == *expected
                    && actual.is_sign_negative() == expected.is_sign_negative())
        }
        (Value::String(_) | Value::StringUnits(_), Value::String(_) | Value::StringUnits(_)) => {
            assert_string_contents(actual) == assert_string_contents(expected)
        }
        (Value::BigInt(actual), Value::BigInt(expected)) => actual == expected,
        (Value::Boolean(actual), Value::Boolean(expected)) => actual == expected,
        (Value::Null, Value::Null) | (Value::Undefined, Value::Undefined) => true,
        _ => assert_same_reference(actual, expected),
    }
}

fn assert_string_contents(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::StringUnits(units) => Some(String::from_utf16_lossy(units)),
        _ => None,
    }
}

/// True only for two references that are the exact same JS object/array/value.
fn assert_same_reference(actual: &Value, expected: &Value) -> bool {
    use std::rc::Rc;
    use Value::*;
    match (actual, expected) {
        (Array(a), Array(b)) => Rc::ptr_eq(a, b),
        (Object(a), Object(b)) => Rc::ptr_eq(a, b),
        (ArrayBuffer(a), ArrayBuffer(b)) => Rc::ptr_eq(a, b),
        (Float64Array(a), Float64Array(b)) => Rc::ptr_eq(a, b),
        (Float32Array(a), Float32Array(b)) => Rc::ptr_eq(a, b),
        (Int8Array(a), Int8Array(b)) => Rc::ptr_eq(a, b),
        (Int16Array(a), Int16Array(b)) => Rc::ptr_eq(a, b),
        (Int32Array(a), Int32Array(b)) => Rc::ptr_eq(a, b),
        (BigInt64Array(a), BigInt64Array(b)) => Rc::ptr_eq(a, b),
        (BigUint64Array(a), BigUint64Array(b)) => Rc::ptr_eq(a, b),
        (Uint32Array(a), Uint32Array(b)) => Rc::ptr_eq(a, b),
        (Uint8Array(a), Uint8Array(b)) => Rc::ptr_eq(a, b),
        (Uint8ClampedArray(a), Uint8ClampedArray(b)) => Rc::ptr_eq(a, b),
        (DataView(a), DataView(b)) => Rc::ptr_eq(a, b),
        (Function(a), Function(b)) => Rc::ptr_eq(a, b),
        (BoundFunction(a), BoundFunction(b)) => Rc::ptr_eq(a, b),
        (Proxy(a), Proxy(b)) => Rc::ptr_eq(a, b),
        (Promise(a), Promise(b)) => Rc::ptr_eq(a, b),
        (HostCapability(a), HostCapability(b)) => Rc::ptr_eq(a, b),
        (Map(a), Map(b)) => Rc::ptr_eq(a, b),
        (Set(a), Set(b)) => Rc::ptr_eq(a, b),
        (Iterator(a), Iterator(b)) => Rc::ptr_eq(a, b),
        (Generator(a), Generator(b)) => Rc::ptr_eq(a, b),
        (BindingCell(a), BindingCell(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Loose (`==`) equality restricted to primitives. Objects/arrays fall through
/// to the caller's reference/container handling.
fn assert_loose_equal(actual: &Value, expected: &Value) -> bool {
    use Value::*;
    let nullish = |value: &Value| matches!(value, Null | Undefined);
    if nullish(actual) || nullish(expected) {
        return nullish(actual) && nullish(expected);
    }
    match (actual, expected) {
        (Number(a), Number(b)) => a == b,
        (String(a), String(b)) => a == b,
        (Boolean(a), Boolean(b)) => a == b,
        (BigInt(a), BigInt(b)) => a == b,
        (Number(a), String(b)) => *a == loose_to_number(b),
        (String(a), Number(b)) => loose_to_number(a) == *b,
        (Number(a), Boolean(b)) => *a == loose_bool(*b),
        (Boolean(a), Number(b)) => loose_bool(*a) == *b,
        (String(a), Boolean(b)) => loose_to_number(a) == loose_bool(*b),
        (Boolean(a), String(b)) => loose_bool(*a) == loose_to_number(b),
        (BigInt(a), String(b)) => big_value(a) == loose_to_number(b),
        (String(a), BigInt(b)) => loose_to_number(a) == big_value(b),
        _ => false,
    }
}

fn loose_bool(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

fn big_value(value: &str) -> f64 {
    value.trim_end_matches('n').parse().unwrap_or(f64::NAN)
}

fn loose_to_number(value: &str) -> f64 {
    let trimmed = value.trim();
    match trimmed {
        "" => 0.0,
        _ => trimmed.parse().unwrap_or(f64::NAN),
    }
}

/// Leaf comparison: strict uses Object.is, loose uses `==` but still treats
/// NaN and ±0 as equal the way Node's deepEqual does.
fn assert_leaf_equal(actual: &Value, expected: &Value, strict: bool) -> bool {
    if strict {
        return assert_object_is(actual, expected);
    }
    matches!(
        (actual, expected),
        (Value::Number(a), Value::Number(b)) if a.is_nan() && b.is_nan()
    ) || assert_loose_equal(actual, expected)
}

/// True when both values are byte/typed-array containers of the same kind.
/// Buffers and plain Uint8Arrays share the `Uint8Array` tag on this engine.
fn assert_same_typed_kind(actual: &Value, expected: &Value) -> bool {
    match (typed_array_name(actual), typed_array_name(expected)) {
        (Some(actual), Some(expected)) => actual == expected,
        _ => matches!(actual, Value::DataView(_)) && matches!(expected, Value::DataView(_)),
    }
}

fn assert_is_typed(actual: &Value) -> bool {
    typed_array_name(actual).is_some() || matches!(actual, Value::DataView(_))
}

/// Compare byte/typed-array content element-wise. Reads indices generically so
/// every typed-array and Buffer shape is handled by the same path.
fn assert_container_equal(actual: &Value, expected: &Value, strict: bool) -> bool {
    let value_length = |value: &Value| {
        quench_runtime::execute::get_property_result(value, "byteLength")
            .ok()
            .and_then(|v| match v {
                Value::Number(n) => Some(n.max(0.0) as usize),
                _ => None,
            })
            .or_else(|| {
                quench_runtime::execute::get_property_result(value, "length")
                    .ok()
                    .and_then(|v| match v {
                        Value::Number(n) => Some(n.max(0.0) as usize),
                        _ => None,
                    })
            })
            .unwrap_or(0)
    };
    let left = value_length(actual);
    if left != value_length(expected) {
        return false;
    }
    (0..left).all(|index| {
        let index = index.to_string();
        let Some(left) = quench_runtime::execute::get_property_result(actual, &index).ok() else {
            return true;
        };
        let Some(right) = quench_runtime::execute::get_property_result(expected, &index).ok()
        else {
            return true;
        };
        assert_leaf_equal(&left, &right, strict)
    })
}

fn assert_is_date_like(value: &Value) -> bool {
    matches!(
        quench_runtime::execute::get_property_result(value, "timeValue"),
        Ok(Value::Number(_))
    )
}

fn assert_is_regexp_like(value: &Value) -> bool {
    matches!(
        quench_runtime::execute::get_property_result(value, "source"),
        Ok(Value::String(_))
    )
}

fn assert_string_value(value: &Value, key: &str) -> Option<String> {
    match quench_runtime::execute::get_property_result(value, key).ok()? {
        Value::String(value) => Some(value),
        _ => None,
    }
}

fn assert_date_equal(actual: &Value, expected: &Value) -> bool {
    let time =
        |value: &Value| match quench_runtime::execute::get_property_result(value, "timeValue") {
            Ok(Value::Number(value)) => Some(value),
            _ => None,
        };
    match (time(actual), time(expected)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn assert_regexp_equal(actual: &Value, expected: &Value) -> bool {
    assert_string_value(actual, "source") == assert_string_value(expected, "source")
        && assert_string_value(actual, "flags") == assert_string_value(expected, "flags")
}

/// Order-insensitive Map comparison: matching keys carry matching values.
fn assert_maps_equal(actual: &Value, expected: &Value, strict: bool) -> bool {
    let (Value::Map(left), Value::Map(right)) = (actual, expected) else {
        return false;
    };
    if left.is_weak() || right.is_weak() {
        return false;
    }
    let left_keys: Vec<_> = left.keys.borrow().iter().cloned().collect();
    let left_values: Vec<_> = left.values.borrow().clone();
    let right_keys: Vec<_> = right.keys.borrow().iter().cloned().collect();
    let right_values: Vec<_> = right.values.borrow().clone();
    if left_keys.len() != right_keys.len() {
        return false;
    }
    let mut used = vec![false; right_keys.len()];
    'outer: for (index, key) in left_keys.iter().enumerate() {
        for (other, (other_key, other_value)) in right_keys.iter().zip(&right_values).enumerate() {
            if !used[other]
                && assert_deep_equal(key, other_key, strict)
                && assert_deep_equal(&left_values[index], other_value, strict)
            {
                used[other] = true;
                continue 'outer;
            }
        }
        return false;
    }
    true
}

/// Order-insensitive Set comparison.
fn assert_sets_equal(actual: &Value, expected: &Value, strict: bool) -> bool {
    let (Value::Set(left), Value::Set(right)) = (actual, expected) else {
        return false;
    };
    if left.is_weak() || right.is_weak() {
        return false;
    }
    let left_values: Vec<_> = left.values.borrow().iter().cloned().collect();
    let right_values: Vec<_> = right.values.borrow().iter().cloned().collect();
    if left_values.len() != right_values.len() {
        return false;
    }
    let mut used = vec![false; right_values.len()];
    'outer: for value in &left_values {
        for (other, other_value) in right_values.iter().enumerate() {
            if !used[other] && assert_deep_equal(value, other_value, strict) {
                used[other] = true;
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn assert_same_constructor(actual: &Value, expected: &Value) -> bool {
    let constructor_name = |value: &Value| {
        quench_runtime::execute::get_property_result(value, "constructor")
            .ok()
            .and_then(|ctor| quench_runtime::execute::get_property_result(&ctor, "name").ok())
            .and_then(|name| match name {
                Value::String(name) => Some(name),
                _ => None,
            })
    };
    match (constructor_name(actual), constructor_name(expected)) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

/// Own-enumerable key comparison for ordinary objects. `strict` additionally
/// requires matching constructors (a lightweight stand-in for prototypes).
fn assert_objects_equal(actual: &Value, expected: &Value, strict: bool) -> bool {
    let (Value::Object(left), Value::Object(right)) = (actual, expected) else {
        return false;
    };
    let left: Vec<_> = left
        .iter()
        .filter(|(key, _)| !key.starts_with('\0'))
        .cloned()
        .collect();
    let right: Vec<_> = right
        .iter()
        .filter(|(key, _)| !key.starts_with('\0'))
        .cloned()
        .collect();
    if left.len() != right.len() {
        return false;
    }
    if strict && !assert_same_constructor(actual, expected) {
        return false;
    }
    left.iter().all(|(key, value)| {
        right
            .iter()
            .find(|(other, _)| other == key)
            .is_some_and(|(_, other)| assert_deep_equal(value, other, strict))
    })
}

/// Recursive deep equality. `strict` selects deepStrictEqual semantics.
fn assert_deep_equal(actual: &Value, expected: &Value, strict: bool) -> bool {
    if let (Some(left), Some(right)) = (url_identity(actual), url_identity(expected)) {
        return left == right;
    }
    match (actual, expected) {
        (Value::Array(_), Value::Array(_)) => {
            let length = quench_runtime::execute::get_property_result(actual, "length")
                .ok()
                .and_then(|length| match length {
                    Value::Number(length) => Some(length.max(0.0) as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let other = quench_runtime::execute::get_property_result(expected, "length")
                .ok()
                .and_then(|length| match length {
                    Value::Number(length) => Some(length.max(0.0) as usize),
                    _ => None,
                })
                .unwrap_or(0);
            length == other
                && (0..length).all(|index| {
                    let index = index.to_string();
                    match (
                        quench_runtime::execute::get_property_result(actual, &index),
                        quench_runtime::execute::get_property_result(expected, &index),
                    ) {
                        (Ok(left), Ok(right)) => assert_deep_equal(&left, &right, strict),
                        _ => true,
                    }
                })
        }
        _ if assert_is_typed(actual)
            && assert_is_typed(expected)
            && assert_same_typed_kind(actual, expected) =>
        {
            assert_container_equal(actual, expected, strict)
        }
        _ if matches!(actual, Value::ArrayBuffer(_))
            && matches!(expected, Value::ArrayBuffer(_)) =>
        {
            assert_container_equal(actual, expected, strict)
        }
        (Value::Map(_), Value::Map(_)) => assert_maps_equal(actual, expected, strict),
        (Value::Set(_), Value::Set(_)) => assert_sets_equal(actual, expected, strict),
        _ if assert_is_date_like(actual) && assert_is_date_like(expected) => {
            assert_date_equal(actual, expected)
        }
        _ if assert_is_regexp_like(actual) && assert_is_regexp_like(expected) => {
            assert_regexp_equal(actual, expected)
        }
        (Value::Object(_), Value::Object(_)) => assert_objects_equal(actual, expected, strict),
        _ => assert_leaf_equal(actual, expected, strict),
    }
}

/// `partialDeepStrictEqual`: every own property of `expected` must deep-equal
/// the corresponding part of `actual`; `actual` may carry extra keys/entries.
fn assert_partial_equal(actual: &Value, expected: &Value) -> bool {
    if let (Value::Object(left), Value::Object(right)) = (actual, expected) {
        let left: Vec<_> = left
            .iter()
            .filter(|(key, _)| !key.starts_with('\0'))
            .cloned()
            .collect();
        let right: Vec<_> = right
            .iter()
            .filter(|(key, _)| !key.starts_with('\0'))
            .cloned()
            .collect();
        return right.iter().all(|(key, value)| {
            left.iter()
                .find(|(other, _)| other == key)
                .is_some_and(|(_, other)| assert_partial_equal(other, value))
        });
    }
    if let (Value::Array(_), Value::Array(_)) = (actual, expected) {
        let value_length = |value: &Value| {
            quench_runtime::execute::get_property_result(value, "length")
                .ok()
                .and_then(|length| match length {
                    Value::Number(length) => Some(length.max(0.0) as usize),
                    _ => None,
                })
                .unwrap_or(0)
        };
        let left = value_length(actual);
        let right = value_length(expected);
        if right > left {
            return false;
        }
        return (0..right).all(|index| {
            let index = index.to_string();
            match (
                quench_runtime::execute::get_property_result(actual, &index),
                quench_runtime::execute::get_property_result(expected, &index),
            ) {
                (Ok(left), Ok(right)) => assert_partial_equal(&left, &right),
                _ => true,
            }
        });
    }
    if let (Value::Set(left), Value::Set(right)) = (actual, expected) {
        if left.is_weak() || right.is_weak() {
            return false;
        }
        let left_values: Vec<_> = left.values.borrow().iter().cloned().collect();
        let right_values: Vec<_> = right.values.borrow().iter().cloned().collect();
        let mut used = vec![false; left_values.len()];
        return right_values.iter().all(|wanted| {
            left_values
                .iter()
                .enumerate()
                .find(|(index, value)| !used[*index] && assert_partial_equal(value, wanted))
                .is_some_and(|(index, _)| {
                    used[index] = true;
                    true
                })
        });
    }
    if assert_is_typed(actual)
        && assert_is_typed(expected)
        && assert_same_typed_kind(actual, expected)
    {
        return assert_container_partial(actual, expected);
    }
    assert_deep_equal(actual, expected, true)
}

/// Expected must be no longer than actual, and prefix must match element-wise.
fn assert_container_partial(actual: &Value, expected: &Value) -> bool {
    let value_length = |value: &Value| {
        quench_runtime::execute::get_property_result(value, "byteLength")
            .ok()
            .and_then(|v| match v {
                Value::Number(n) => Some(n.max(0.0) as usize),
                _ => None,
            })
            .or_else(|| {
                quench_runtime::execute::get_property_result(value, "length")
                    .ok()
                    .and_then(|v| match v {
                        Value::Number(n) => Some(n.max(0.0) as usize),
                        _ => None,
                    })
            })
            .unwrap_or(0)
    };
    let left = value_length(actual);
    let right = value_length(expected);
    if right > left {
        return false;
    }
    (0..right).all(|index| {
        let index = index.to_string();
        match (
            quench_runtime::execute::get_property_result(actual, &index),
            quench_runtime::execute::get_property_result(expected, &index),
        ) {
            (Ok(left), Ok(right)) => assert_leaf_equal(&left, &right, true),
            _ => true,
        }
    })
}
