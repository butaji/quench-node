fn is_callable_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Function(_) | Value::BoundFunction(_) | Value::HostCapability(_) | Value::Proxy(_)
    )
}

fn regex_matches(pattern: &Value, string: &str) -> bool {
    let Ok(test) = quench_runtime::execute::get_property_result(pattern, "test") else {
        return regex_source_matches(pattern, string);
    };
    if is_callable_value(&test) {
        if matches!(
            quench_runtime::execute::call(&test, pattern, &[Value::String(string.into())]),
            Ok(Value::Boolean(true))
        ) {
            return true;
        }
    }
    regex_source_matches(pattern, string)
}

fn regex_source_matches(pattern: &Value, string: &str) -> bool {
    let Some(source) = assert_string_value(pattern, "source") else {
        if let Value::String(source) = pattern {
            return rust_regex_is_match(source, string) || string.contains(source.as_str());
        }
        return false;
    };
    rust_regex_is_match(&source, string)
}

fn rust_regex_is_match(source: &str, string: &str) -> bool {
    let source = source.replace(r"\/", "/");
    regex::Regex::new(&source)
        .map(|pattern| pattern.is_match(string))
        .unwrap_or(false)
}

/// `assert.throws(fn, expected)`: constructor, validator, RegExp, or object.
fn expected_matches(received: &Value, expected: &Value) -> bool {
    if assert_is_regexp_like(expected) {
        return regex_matches(expected, &error_text(received));
    }
    if is_callable_value(expected) {
        return expected_callable_matches(received, expected);
    }
    let Value::Object(object) = expected else {
        return true;
    };
    object
        .iter()
        .filter(|(key, _)| !key.starts_with('\0'))
        .all(|(key, wanted)| {
            let Ok(received) = quench_runtime::execute::get_property_result(received, key) else {
                return false;
            };
            expected_value_matches(&received, wanted)
        })
}

fn expected_callable_matches(received: &Value, expected: &Value) -> bool {
    match quench_runtime::execute::call(expected, &Value::Undefined, std::slice::from_ref(received))
    {
        Ok(Value::Boolean(matched)) => return matched,
        Ok(_) => {}
        Err(_) => return false,
    }
    let received_name = assert_string_value(received, "name");
    let expected_name = assert_string_value(expected, "name");
    expected_name.is_none()
        || received_name == expected_name
        || matches!(received_name.as_deref(), Some("AssertionError"))
            && matches!(
                expected_name.as_deref(),
                Some("Error") | Some("AssertionError")
            )
}

fn expected_value_matches(received: &Value, wanted: &Value) -> bool {
    if assert_is_regexp_like(wanted) {
        return regex_matches(wanted, &safe_value_string(received));
    }
    assert_object_is(received, wanted) || assert_loose_equal(received, wanted)
}

fn assertion_if_error(arguments: &[Value]) -> Result<Value, VmError> {
    match arguments.first() {
        None | Some(Value::Null) | Some(Value::Undefined) => Ok(Value::Undefined),
        Some(value) => {
            let detail = if_error_detail(value);
            assertion_fail(
                "ifError",
                &format!("ifError got unwanted exception: {detail}"),
                Some(value),
                Some(&Value::Null),
                None,
            )
        }
    }
}

fn if_error_detail(value: &Value) -> String {
    if let Some(message) = assert_string_value(value, "message") {
        if message.is_empty() {
            return assert_string_value(value, "name").unwrap_or_else(|| "Error".into());
        }
        return message;
    }
    format_inspected(value)
}

fn assertion_fail_call(arguments: &[Value]) -> Result<Value, VmError> {
    match arguments.first() {
        Some(value) if assert_string_value(value, "message").is_some() => {
            Err(VmError::Thrown(value.clone()))
        }
        Some(Value::String(message)) => assertion_fail(
            "fail",
            message,
            Some(&Value::Undefined),
            Some(&Value::Undefined),
            Some(arguments.first().unwrap()),
        ),
        Some(value) => assertion_fail(
            "fail",
            "Failed",
            Some(&Value::Undefined),
            Some(&Value::Undefined),
            Some(value),
        ),
        None => assertion_fail(
            "fail",
            "Failed",
            Some(&Value::Undefined),
            Some(&Value::Undefined),
            None,
        ),
    }
}
