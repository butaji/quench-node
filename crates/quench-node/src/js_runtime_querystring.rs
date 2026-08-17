fn querystring_parse(receiver: Option<&Value>, arguments: &[Value]) -> Result<Value, VmError> {
    let Some(Value::String(input)) = arguments.first() else {
        return Ok(quench_runtime::host_api::object(vec![]));
    };
    let separator = querystring_option_string(arguments.get(1), "&");
    let equals = querystring_option_string(arguments.get(2), "=");
    let max_keys = arguments
        .get(3)
        .and_then(|options| quench_runtime::execute::get_property_result(options, "maxKeys").ok())
        .map_or(1000, |value| match value {
            Value::Number(value) if value.is_nan() || value.is_infinite() => usize::MAX,
            Value::Number(value) if value > 0.0 => value as usize,
            _ => 1000,
        });
    let decoder = arguments
        .get(3)
        .and_then(|options| {
            quench_runtime::execute::get_property_result(options, "decodeURIComponent")
                .ok()
                .filter(|value| {
                    matches!(
                        value,
                        Value::Function(_) | Value::BoundFunction(_) | Value::Builtin(_)
                    )
                })
        })
        .or_else(|| {
            receiver.and_then(|receiver| {
                quench_runtime::execute::get_property_result(receiver, "unescape")
                    .ok()
                    .filter(|value| {
                        matches!(
                            value,
                            Value::Function(_) | Value::BoundFunction(_) | Value::Builtin(_)
                        )
                    })
            })
        });
    let mut properties: Vec<(String, Value)> = Vec::new();
    let mut property_indices: HashMap<String, usize> = HashMap::new();
    for pair in input
        .split(&separator)
        .take(max_keys)
        .filter(|pair| !pair.is_empty())
    {
        let (key, value) = pair.split_once(&equals).unwrap_or((pair, ""));
        let key = querystring_apply_decoder(&querystring_decode(key), decoder.as_ref());
        let value = Value::String(
            querystring_apply_decoder(&querystring_decode(value), decoder.as_ref()).into(),
        );
        if let Some(index) = property_indices.get(&key).copied() {
            let existing = &mut properties[index].1;
            *existing = match existing.clone() {
                Value::Array(array) => {
                    let mut values = Vec::new();
                    for index in 0..array_length(&Value::Array(array.clone())) {
                        if let Ok(value) = quench_runtime::execute::get_property_result(
                            &Value::Array(array.clone()),
                            &index.to_string(),
                        ) {
                            values.push(value);
                        }
                    }
                    values.push(value);
                    Value::Array(Rc::new(quench_runtime::value::ArrayData::new(values)))
                }
                other => Value::Array(Rc::new(quench_runtime::value::ArrayData::new(vec![
                    other, value,
                ]))),
            };
        } else {
            property_indices.insert(key.clone(), properties.len());
            properties.push((key, value));
        }
    }
    properties.insert(0, ("\0prototype".into(), Value::Null));
    Ok(Value::object(properties))
}

fn querystring_option_string(value: Option<&Value>, default: &str) -> String {
    match value {
        None | Some(Value::Null) | Some(Value::Undefined) => default.into(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Array(array)) if array_length(&Value::Array(array.clone())) == 0 => {
            String::new()
        }
        Some(value) => safe_value_string(value),
    }
}

fn array_length(value: &Value) -> usize {
    quench_runtime::execute::get_property_result(value, "length")
        .ok()
        .and_then(|value| match value {
            Value::Number(length) => Some(length as usize),
            _ => None,
        })
        .unwrap_or(0)
}

fn querystring_decode(value: &str) -> String {
    String::from_utf8_lossy(&querystring_decode_bytes(value)).into_owned()
}

fn querystring_decode_bytes(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'+' {
            output.push(b' ');
        } else if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            };
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                output.push(high * 16 + low);
                index += 2;
            } else {
                output.push(b'%');
            }
        } else {
            output.push(bytes[index]);
        }
        index += 1;
    }
    output
}

fn querystring_escape(arguments: &[Value]) -> Result<Value, VmError> {
    let value = arguments.first().cloned().unwrap_or(Value::Undefined);
    match querystring_coerce_string(&value)? {
        Value::String(text) => Ok(Value::String(querystring_encode(&text).into())),
        Value::StringUnits(units) => querystring_encode_units(&units),
        _ => Ok(Value::String("".into())),
    }
}

/// Coerce a value to a JS string following `String(value)` semantics (the
/// object path uses `toString`/`valueOf` and throws a TypeError when neither
/// yields a primitive). Surrogate-bearing input is kept as `StringUnits`.
fn querystring_coerce_string(value: &Value) -> Result<Value, VmError> {
    match value {
        Value::StringUnits(_) => Ok(value.clone()),
        Value::Array(_) => {
            let text = quench_runtime::execute::get_property_result(value, "length")
                .map(|length| {
                    let count = match length {
                        Value::Number(count) => count.max(0.0) as usize,
                        _ => 0,
                    };
                    let mut parts = Vec::new();
                    for index in 0..count {
                        if let Ok(item) =
                            quench_runtime::execute::get_property_result(value, &index.to_string())
                        {
                            if let Ok(coerced) = querystring_coerce_string(&item) {
                                if let Ok(text) = querystring_value_text(&coerced) {
                                    parts.push(text);
                                }
                            }
                        }
                    }
                    parts.join(",")
                })
                .unwrap_or_default();
            Ok(Value::String(text.into()))
        }
        Value::Object(_) | Value::ObjectAlias(_) | Value::Proxy(_) | Value::Function(_)
        | Value::BoundFunction(_) => querystring_call_to_primitive(value),
        Value::String(text) if is_symbol_representation(text) => {
            Err(VmError::Thrown(quench_runtime::host_api::object(vec![
                ("name".into(), Value::String("TypeError".into())),
            ])))
        }
        other => Ok(Value::String(safe_value_string(other).into())),
    }
}

fn querystring_value_text(value: &Value) -> Result<String, VmError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::StringUnits(units) => {
            let mut out = String::new();
            for &unit in units.iter() {
                out.push(char::from_u32(unit as u32).unwrap_or('\u{FFFD}'));
            }
            Ok(out)
        }
        _ => Ok(String::new()),
    }
}

/// Call `toString` (then `valueOf`) on an object; throw a TypeError when the
/// result is neither a string nor a primitive.
fn querystring_call_to_primitive(value: &Value) -> Result<Value, VmError> {
    for method in ["toString", "valueOf"] {
        let Some(function) = quench_runtime::execute::get_property_result(value, method).ok()
        else {
            continue;
        };
        if matches!(
            function,
            Value::Function(_) | Value::BoundFunction(_) | Value::Builtin(_)
        ) {
            if let Ok(result) = quench_runtime::execute::call(&function, value, &[]) {
                match result {
                    Value::String(_) => return Ok(result),
                    Value::StringUnits(_) => return Ok(result),
                    Value::Number(_) | Value::Boolean(_) | Value::BigInt(_) => {
                        return Ok(Value::String(safe_value_string(&result).into()));
                    }
                    Value::Null | Value::Undefined => {
                        return Ok(Value::String("null".into()));
                    }
                    _ => continue,
                }
            }
        }
    }
    Err(VmError::Thrown(quench_runtime::host_api::object(vec![
        ("name".into(), Value::String("TypeError".into())),
    ])))
}

/// Percent-encode a string of raw UTF-16 code units, matching Node's
/// surrogate-pair handling for `encodeStr`.
fn querystring_encode_units(units: &std::rc::Rc<Vec<u16>>) -> Result<Value, VmError> {
    let mut output = String::new();
    let len = units.len();
    let mut i = 0usize;
    while i < len {
        let unit = units[i];
        let mut c = unit as u32;
        if c < 0x80 {
            let byte = unit as u8;
            if byte.is_ascii_alphanumeric() || b"-._~!*'()".contains(&byte) {
                output.push(byte as char);
            } else {
                output.push_str(&format!("%{byte:02X}"));
            }
            i += 1;
        } else if c < 0x800 {
            output.push_str(&format!(
                "%{:02X}%{:02X}",
                0xC0 | (c >> 6),
                0x80 | (c & 0x3f)
            ));
            i += 1;
        } else if c < 0xD800 || c >= 0xE000 {
            output.push_str(&format!(
                "%{:02X}%{:02X}%{:02X}",
                0xE0 | (c >> 12),
                0x80 | ((c >> 6) & 0x3f),
                0x80 | (c & 0x3f)
            ));
            i += 1;
        } else {
            // Surrogate: consume the following code unit as the low half.
            i += 1;
            if i >= len {
                return Err(VmError::Thrown(quench_runtime::host_api::object(vec![
                    ("code".into(), Value::String("ERR_INVALID_URI".into())),
                    ("name".into(), Value::String("URIError".into())),
                    ("message".into(), Value::String("URI malformed".into())),
                ])));
            }
            let c2 = (units[i] as u32) & 0x3FF;
            c = 0x10000 + (((c & 0x3FF) << 10) | c2);
            output.push_str(&format!(
                "%{:02X}%{:02X}%{:02X}%{:02X}",
                0xF0 | (c >> 18),
                0x80 | ((c >> 12) & 0x3f),
                0x80 | ((c >> 6) & 0x3f),
                0x80 | (c & 0x3f)
            ));
            i += 1;
        }
    }
    Ok(Value::String(output.into()))
}

fn querystring_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~!*'()".contains(byte) {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn querystring_unescape_buffer(arguments: &[Value]) -> Result<Value, VmError> {
    let input = match arguments.first() {
        Some(Value::String(value)) => value.as_str(),
        _ => "",
    };
    let decode_spaces = matches!(arguments.get(1), Some(Value::Boolean(true)));
    let input = if decode_spaces {
        input.to_owned()
    } else {
        input.replace('+', "%2B")
    };
    Ok(node_buffer(&querystring_decode_bytes(&input)))
}

fn querystring_stringify(arguments: &[Value]) -> Result<Value, VmError> {
    let Some(Value::Object(object)) = arguments.first() else {
        return Ok(Value::String(String::new().into()));
    };
    let separator = match arguments.get(1) {
        Some(Value::String(value)) => value.as_str(),
        _ => "&",
    };
    let equals = match arguments.get(2) {
        Some(Value::String(value)) => value.as_str(),
        _ => "=",
    };
    let encoder = arguments.get(3).and_then(|options| {
        quench_runtime::execute::get_property_result(options, "encodeURIComponent")
            .ok()
            .filter(|value| {
                matches!(
                    value,
                    Value::Function(_) | Value::BoundFunction(_) | Value::Builtin(_)
                )
            })
    });
    let mut pairs = Vec::new();
    let keys = quench_runtime::execute::call(
        &Value::Builtin(quench_runtime::ops::Builtin::ObjectKeys),
        &Value::Undefined,
        &[Value::Object(object.clone())],
    )?;
    let key_count = match quench_runtime::execute::get_property_result(&keys, "length")? {
        Value::Number(length) => length as usize,
        _ => 0,
    };
    for index in 0..key_count {
        let key = match quench_runtime::execute::get_property_result(&keys, &index.to_string())? {
            Value::String(key) => key,
            _ => continue,
        };
        let value =
            quench_runtime::execute::get_property_result(&Value::Object(object.clone()), &key)?;
        let values = if matches!(&value, Value::Array(_)) {
            let length = quench_runtime::execute::get_property_result(&value, "length")
                .ok()
                .and_then(|value| match value {
                    Value::Number(length) => Some(length as usize),
                    _ => None,
                })
                .unwrap_or(0);
            (0..length)
                .filter_map(|index| {
                    quench_runtime::execute::get_property_result(&value, &index.to_string()).ok()
                })
                .collect()
        } else {
            vec![value.clone()]
        };
        for value in values {
            let rendered = match value {
                Value::StringUnits(_) => {
                    return Err(VmError::Thrown(quench_runtime::host_api::object(vec![
                        ("name".into(), Value::String("URIError".into())),
                        ("code".into(), Value::String("ERR_INVALID_URI".into())),
                        ("message".into(), Value::String("URI malformed".into())),
                        (
                            "constructor".into(),
                            Value::Builtin(quench_runtime::ops::Builtin::URIError),
                        ),
                    ])));
                }
                Value::Null
                | Value::Undefined
                | Value::Object(_)
                | Value::ObjectAlias(_)
                | Value::Function(_)
                | Value::BoundFunction(_) => String::new(),
                Value::Number(number) if !number.is_finite() => String::new(),
                Value::BigInt(value) => querystring_apply_encoder(
                    &Value::String(value.trim_end_matches('n').to_owned()),
                    encoder.as_ref(),
                    &querystring_encode(value.trim_end_matches('n')),
                ),
                other => querystring_encode(&safe_value_string(&other)),
            };
            let encoded_key = querystring_apply_encoder(
                &Value::String(key.clone()),
                encoder.as_ref(),
                &querystring_encode(&key),
            );
            pairs.push(format!("{}{}{}", encoded_key, equals, rendered));
        }
    }
    Ok(Value::String(pairs.join(separator).into()))
}

fn querystring_apply_encoder(value: &Value, encoder: Option<&Value>, fallback: &str) -> String {
    encoder
        .and_then(|encoder| {
            quench_runtime::execute::call(encoder, &Value::Undefined, &[value.clone()]).ok()
        })
        .map(|value| safe_value_string(&value))
        .unwrap_or_else(|| fallback.to_owned())
}

fn querystring_apply_decoder(value: &str, decoder: Option<&Value>) -> String {
    decoder
        .and_then(|decoder| {
            quench_runtime::execute::call(
                decoder,
                &Value::Undefined,
                &[Value::String(value.to_owned())],
            )
            .ok()
        })
        .map(|value| safe_value_string(&value))
        .unwrap_or_else(|| value.to_owned())
}
