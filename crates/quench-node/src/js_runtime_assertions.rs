fn assertion_call(id: u16, arguments: &[Value]) -> Result<Value, VmError> {
    match id {
        13 | 16 => {
            if arguments.first().is_some_and(is_truthy) {
                Ok(Value::Undefined)
            } else {
                assertion_fail(
                    "ok",
                    "The expression evaluated to a falsy value",
                    arguments.first(),
                    None,
                    arguments.get(1),
                )
            }
        }
        14 => assertion_cmp(
            arguments,
            "strictEqual",
            "Expected values to be strictly equal",
            true,
            |a, e| assert_object_is(a, e),
        ),
        24 => assertion_cmp(
            arguments,
            "notStrictEqual",
            "Expected \"actual\" to be strictly unequal to \"expected\"",
            true,
            |a, e| !assert_object_is(a, e),
        ),
        15 => assertion_cmp(
            arguments,
            "deepStrictEqual",
            "Expected values to be strictly deep-equal",
            true,
            |a, e| assert_deep_equal(a, e, true),
        ),
        25 => assertion_cmp(
            arguments,
            "notDeepStrictEqual",
            "Expected \"actual\" not to be strictly deep-equal to \"expected\"",
            true,
            |a, e| !assert_deep_equal(a, e, true),
        ),
        39 => assertion_cmp(
            arguments,
            "deepEqual",
            "Expected values to be loosely deep-equal",
            true,
            |a, e| assert_deep_equal(a, e, false),
        ),
        38 => assertion_cmp(
            arguments,
            "notDeepEqual",
            "Expected \"actual\" not to be loosely deep-equal to \"expected\"",
            true,
            |a, e| !assert_deep_equal(a, e, false),
        ),
        33 => assertion_cmp(
            arguments,
            "equal",
            "Expected values to be loosely equal",
            true,
            |a, e| assert_loose_equal(a, e),
        ),
        34 => assertion_cmp(
            arguments,
            "notEqual",
            "Expected \"actual\" not to be loosely equal to \"expected\"",
            true,
            |a, e| !assert_loose_equal(a, e),
        ),
        17 => assertion_throws(arguments),
        18 => assertion_does_not_throw(arguments),
        19 => assertion_if_error(arguments),
        35 => {
            let string = arguments.first().map(safe_value_string).unwrap_or_default();
            let pattern = arguments.get(1).cloned().unwrap_or(Value::Undefined);
            if regex_matches(&pattern, &string) {
                Ok(Value::Undefined)
            } else {
                assertion_fail(
                    "match",
                    "The input did not match the regular expression",
                    arguments.first(),
                    arguments.get(1),
                    arguments.get(2),
                )
            }
        }
        36 => assertion_fail_call(arguments),
        37 => {
            let string = arguments.first().map(safe_value_string).unwrap_or_default();
            let pattern = arguments.get(1).cloned().unwrap_or(Value::Undefined);
            if regex_matches(&pattern, &string) {
                assertion_fail(
                    "doesNotMatch",
                    "The input matched the regular expression",
                    arguments.first(),
                    arguments.get(1),
                    arguments.get(2),
                )
            } else {
                Ok(Value::Undefined)
            }
        }
        20 => Ok(Value::Undefined),
        26 => assertion_error_construct(arguments),
        41 => assertion_partial_call(arguments),
        42 => assertion_class_call(),
        _ => Err(VmError::NotCallable),
    }
}

/// Shared pass/fail wrapper for a comparison predicate.
fn assertion_cmp(
    arguments: &[Value],
    operator: &str,
    message: &str,
    pass_on_true: bool,
    predicate: impl Fn(&Value, &Value) -> bool,
) -> Result<Value, VmError> {
    let Some(actual) = arguments.get(0) else {
        return assertion_missing_args(operator);
    };
    let Some(expected) = arguments.get(1) else {
        return assertion_missing_args(operator);
    };
    if predicate(actual, expected) == pass_on_true {
        Ok(Value::Undefined)
    } else {
        assertion_fail(
            operator,
            message,
            Some(actual),
            Some(expected),
            arguments.get(2),
        )
    }
}

fn assertion_missing_args(operator: &str) -> Result<Value, VmError> {
    assertion_fail(
        operator,
        "The \"actual\" and \"expected\" arguments must be specified",
        None,
        None,
        None,
    )
}

fn assertion_throws(arguments: &[Value]) -> Result<Value, VmError> {
    let Some(callback) = arguments.first() else {
        return assertion_fail(
            "throws",
            "The \"block\" argument must be specified",
            None,
            None,
            None,
        );
    };
    match quench_runtime::execute::call(callback, &Value::Undefined, &[]) {
        Ok(_) => assertion_fail(
            "throws",
            "Missing expected exception",
            None,
            arguments.get(1),
            arguments.get(2),
        ),
        Err(error) => {
            let thrown =
                thrown_value(&error).unwrap_or_else(|| Value::String(format!("{error:?}")));
            if let Some(expected) = arguments.get(1) {
                if !expected_matches(&thrown, expected) {
                    return assertion_fail(
                        "throws",
                        "The error did not match the expected value",
                        Some(&thrown),
                        Some(expected),
                        arguments.get(2),
                    );
                }
            }
            Ok(Value::Undefined)
        }
    }
}

fn assertion_does_not_throw(arguments: &[Value]) -> Result<Value, VmError> {
    let Some(callback) = arguments.first() else {
        return assertion_fail(
            "doesNotThrow",
            "The \"block\" argument must be specified",
            None,
            None,
            None,
        );
    };
    match quench_runtime::execute::call(callback, &Value::Undefined, &[]) {
        Ok(_) => Ok(Value::Undefined),
        Err(error) => {
            if let Some(expected) = arguments.get(1) {
                if let Some(received) = thrown_value(&error) {
                    if expected_matches(&received, expected) {
                        return assertion_fail(
                            "doesNotThrow",
                            "Got unwanted exception",
                            Some(&received),
                            Some(expected),
                            None,
                        );
                    }
                }
            }
            Err(error)
        }
    }
}

fn error_text(value: &Value) -> String {
    if let Some(message) = assert_string_value(value, "message") {
        let name = assert_string_value(value, "name").unwrap_or_else(|| "Error".into());
        if message.is_empty() {
            return name;
        }
        return format!("{name}: {message}");
    }
    if let Some(stack) = assert_string_value(value, "stack") {
        return stack;
    }
    safe_value_string(value)
}

fn thrown_value(error: &VmError) -> Option<Value> {
    match error {
        VmError::Thrown(value) => Some(value.clone()),
        _ => None,
    }
}

fn assertion_partial_call(arguments: &[Value]) -> Result<Value, VmError> {
    let Some(actual) = arguments.get(0) else {
        return assertion_missing_args("partialDeepStrictEqual");
    };
    let Some(expected) = arguments.get(1) else {
        return assertion_missing_args("partialDeepStrictEqual");
    };
    if assert_partial_equal(actual, expected) {
        Ok(Value::Undefined)
    } else {
        assertion_fail(
            "partialDeepStrictEqual",
            "Expected subset was not found",
            Some(actual),
            Some(expected),
            arguments.get(2),
        )
    }
}

/// `Assert` invoked without `new` fails like a class constructor. (`new`
/// reaches the engine's construct path, which host capabilities do not model.)
fn assertion_error_construct(arguments: &[Value]) -> Result<Value, VmError> {
    let message = arguments
        .first()
        .map(safe_value_string)
        .unwrap_or_else(|| "Failed".into());
    Ok(assertion_error("fail", &message, None, None, false))
}

fn assertion_class_call() -> Result<Value, VmError> {
    let error = Value::object(vec![
        (
            "code".into(),
            Value::String("ERR_CONSTRUCT_CALL_REQUIRED".into()),
        ),
        (
            "name".into(),
            Value::String("Class constructor Assert cannot be invoked without 'new'".into()),
        ),
    ]);
    Err(VmError::Thrown(error))
}

fn assertion_fail(
    operator: &str,
    message: &str,
    actual: Option<&Value>,
    expected: Option<&Value>,
    user: Option<&Value>,
) -> Result<Value, VmError> {
    let generated = user.is_none();
    let message = match user {
        Some(Value::String(value)) => value.clone(),
        Some(value) => safe_value_string(value),
        None => message.to_string(),
    };
    Err(VmError::Thrown(assertion_error(
        operator, &message, actual, expected, generated,
    )))
}

fn assertion_error(
    operator: &str,
    message: &str,
    actual: Option<&Value>,
    expected: Option<&Value>,
    generated: bool,
) -> Value {
    let mut properties = vec![
        ("name".into(), Value::String("AssertionError".into())),
        ("code".into(), Value::String("ERR_ASSERTION".into())),
        ("operator".into(), Value::String(operator.into())),
        ("message".into(), Value::String(message.into())),
        (
            "stack".into(),
            Value::String(format!("AssertionError: {message}")),
        ),
    ];
    properties.push(("actual".into(), actual.cloned().unwrap_or(Value::Undefined)));
    properties.push((
        "expected".into(),
        expected.cloned().unwrap_or(Value::Undefined),
    ));
    properties.push(("generatedMessage".into(), Value::Boolean(generated)));
    Value::object(properties)
}

fn assertion_error_value(message: &str) -> Value {
    Value::object(vec![
        ("name".into(), Value::String("AssertionError".into())),
        ("message".into(), Value::String(message.into())),
        ("code".into(), Value::String("ERR_ASSERTION".into())),
    ])
}

fn rejection_matches(reason: &Value, expected: Option<&Value>) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    let Value::Object(object) = expected else {
        return true;
    };
    object
        .iter()
        .filter(|(key, _)| !key.starts_with('\0'))
        .all(|(key, wanted)| {
            let wanted = safe_value_string(wanted);
            quench_runtime::execute::get_property_result(reason, &key)
                .map(|received| safe_value_string(&received) == wanted)
                .unwrap_or(false)
        })
}

fn assert_rejects_call(arguments: &[Value], expect_reject: bool) -> Result<Value, VmError> {
    let Some(first) = arguments.first().cloned() else {
        return Ok(rejected(assertion_error_value("promiseFn is required")));
    };
    let input = if matches!(
        first,
        Value::Function(_) | Value::BoundFunction(_) | Value::HostCapability(_)
    ) {
        quench_runtime::execute::call(&first, &Value::Undefined, &[]).unwrap_or(Value::Undefined)
    } else {
        first
    };
    let Value::Promise(promise) = input else {
        return Ok(rejected(assertion_error_value(
            "Expected instance of Promise",
        )));
    };
    let state = promise.state.borrow().clone();
    match state {
        quench_runtime::value::PromiseState::Rejected(reason) => {
            if expect_reject == rejection_matches(&reason, arguments.get(1)) {
                Ok(fulfilled(Value::Undefined))
            } else {
                Ok(rejected(assertion_error_value("The input did not match")))
            }
        }
        quench_runtime::value::PromiseState::Fulfilled(_) => {
            if expect_reject {
                Ok(rejected(assertion_error_value(
                    "Missing expected rejection",
                )))
            } else {
                Ok(fulfilled(Value::Undefined))
            }
        }
        quench_runtime::value::PromiseState::Pending => Err(VmError::EvalError(
            "assert.rejects: promise is pending (resolved asynchronously)".into(),
        )),
    }
}
