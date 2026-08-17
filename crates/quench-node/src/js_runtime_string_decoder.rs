fn string_decoder_module() -> Value {
    let constructor = capability_function(HostCapabilityKind::Custom(
        CapabilityName::StringDecoderConstructor,
    ));
    let constructor = quench_runtime::execute::set_property(
        constructor,
        "call",
        capability_function(HostCapabilityKind::Custom(
            CapabilityName::StringDecoderCall,
        )),
    );
    quench_runtime::host_api::object(vec![("StringDecoder".into(), constructor)])
}

fn string_decoder_object(encoding: &str) -> Value {
    let encoding = encoding.to_ascii_lowercase().replace('-', "");
    let encoding = if encoding.is_empty() {
        "utf8".to_owned()
    } else {
        encoding
    };
    quench_runtime::host_api::object(vec![
        ("encoding".into(), Value::String(encoding.into())),
        (
            "_pending".into(),
            Value::BindingCell(Rc::new(RefCell::new(quench_runtime::host_api::array(
                Vec::new(),
            )))),
        ),
        (
            "lastNeed".into(),
            Value::BindingCell(Rc::new(RefCell::new(Value::Number(0.0)))),
        ),
        (
            "lastTotal".into(),
            Value::BindingCell(Rc::new(RefCell::new(Value::Number(0.0)))),
        ),
        (
            "lastChar".into(),
            Value::BindingCell(Rc::new(RefCell::new(node_buffer(&[0, 0, 0, 0])))),
        ),
        (
            "write".into(),
            capability_function(HostCapabilityKind::Custom(
                CapabilityName::StringDecoderWrite,
            )),
        ),
        (
            "end".into(),
            capability_function(HostCapabilityKind::Custom(CapabilityName::StringDecoderEnd)),
        ),
        (
            "text".into(),
            capability_function(HostCapabilityKind::Custom(
                CapabilityName::StringDecoderText,
            )),
        ),
    ])
}

fn string_decoder_constructor(
    receiver: Option<&Value>,
    arguments: &[Value],
) -> Result<Value, VmError> {
    let encoding = match arguments.first() {
        None | Some(Value::Undefined) => "utf8".to_owned(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => safe_value_string(value),
    };
    let normalized = encoding.to_ascii_lowercase().replace('-', "");
    if !matches!(
        normalized.as_str(),
        "utf8" | "ucs2" | "utf16le" | "latin1" | "binary" | "ascii" | "base64" | "base64url" | "hex"
    ) {
        return Err(VmError::Thrown(fs_error(
            "ERR_UNKNOWN_ENCODING",
            &format!("Unknown encoding: {encoding}"),
        )));
    }
    let object = string_decoder_object(&normalized);
    if let Some(receiver) = receiver {
        quench_runtime::execute::replace_value(receiver, &object);
        for key in [
            "encoding",
            "_pending",
            "lastNeed",
            "lastTotal",
            "lastChar",
            "write",
            "end",
            "text",
        ] {
            if let Ok(value) = quench_runtime::execute::get_property_result(&object, key) {
                let _ = quench_runtime::execute::set_property(receiver.clone(), key, value);
            }
        }
        return Ok(object);
    }
    Ok(object)
}

fn string_decoder_bytes(value: &Value) -> Result<Vec<u8>, VmError> {
    let bytes = match value {
        Value::Uint16Array(view) => view.buffer.bytes.borrow()
            [view.byte_offset..view.byte_offset + view.length * 2]
            .to_vec(),
        Value::Uint32Array(view) => view.buffer.bytes.borrow()
            [view.byte_offset..view.byte_offset + view.length * 4]
            .to_vec(),
        _ => string_or_bytes(Some(value)).map_err(|_| {
            VmError::Thrown(fs_error(
                "ERR_INVALID_ARG_TYPE",
                "The \"buf\" argument must be an instance of Buffer, TypedArray, or DataView.",
            ))
        })?,
    };
    Ok(bytes)
}

/// Base64-encode raw bytes. When `url` is set, uses the URL-safe alphabet and
/// omits padding (Node's `base64url`).
fn sd_base64_encode(bytes: &[u8], url: bool) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(ALPHABET[(a >> 2) as usize] as char);
        out.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    if url {
        out = out.replace('+', "-").replace('/', "_");
        while out.ends_with('=') {
            out.pop();
        }
    }
    out
}

fn sd_hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

fn string_decoder_write(receiver: Option<&Value>, arguments: &[Value]) -> Result<Value, VmError> {
    let receiver = receiver.ok_or(VmError::NotCallable)?;
    let input = arguments.first().ok_or(VmError::NotCallable)?;
    let mut bytes = quench_runtime::execute::get_property_result(receiver, "_pending")
        .ok()
        .and_then(|value| array_values(&value).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| match value {
            Value::Number(value) => Some(value as u8),
            _ => None,
        })
        .collect::<Vec<_>>();
    bytes.extend(string_decoder_bytes(input)?);
    let encoding = quench_runtime::execute::get_property_result(receiver, "encoding")
        .ok()
        .and_then(|value| match value {
            Value::String(value) => Some(value),
            _ => None,
        })
        .unwrap_or_else(|| "utf8".into());
    let (text_value, pending): (Value, Vec<u8>) = if encoding == "utf16le" || encoding == "ucs2" {
        let mut complete = bytes.len() / 2 * 2;
        if complete >= 2 {
            let last = u16::from_le_bytes([bytes[complete - 2], bytes[complete - 1]]);
            if (0xd800..=0xdbff).contains(&last) {
                complete -= 2;
            }
        }
        let units = bytes[..complete]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<u16>>();
        let text_value = if units.iter().any(|unit| (0xd800..=0xdfff).contains(unit)) {
            Value::StringUnits(Rc::new(units))
        } else {
            Value::String(String::from_utf16(&units).unwrap_or_default().into())
        };
        (text_value, bytes[complete..].to_vec())
    } else if encoding == "base64" {
        let complete = bytes.len() / 3 * 3;
        (
            Value::String(sd_base64_encode(&bytes[..complete], false).into()),
            bytes[complete..].to_vec(),
        )
    } else if encoding == "base64url" {
        let complete = bytes.len() / 3 * 3;
        (
            Value::String(sd_base64_encode(&bytes[..complete], true).into()),
            bytes[complete..].to_vec(),
        )
    } else if encoding == "hex" {
        (Value::String(sd_hex_encode(&bytes).into()), Vec::new())
    } else if encoding == "latin1" || encoding == "binary" || encoding == "ascii" {
       (
            Value::String(
                bytes
                    .iter()
                    .map(|byte| {
                        char::from(if encoding == "ascii" {
                            byte & 0x7f
                        } else {
                            *byte
                        })
                    })
                    .collect(),
            ),
            Vec::new(),
        )
    } else {
        match String::from_utf8(bytes.clone()) {
            Ok(text) => (Value::String(text.into()), Vec::new()),
            Err(error) if error.utf8_error().error_len().is_some() => {
                (Value::String(String::from_utf8_lossy(&bytes).into_owned().into()), Vec::new())
            }
            Err(error) => {
                let valid = error.utf8_error().valid_up_to();
                let pending = bytes.split_off(valid);
                (Value::String(String::from_utf8_lossy(&bytes).into_owned().into()), pending)
            }
        }
    };
    let pending = quench_runtime::host_api::array(
        pending
            .into_iter()
            .map(|byte| Value::Number(byte as f64))
            .collect(),
    );
    let pending_values = array_values(&pending).unwrap_or_default();
    let _ = quench_runtime::execute::set_property(
        receiver.clone(),
        "lastNeed",
        Value::Number(if pending_values.is_empty() {
            0.0
        } else {
            (3 - pending_values.len()) as f64
        }),
    );
    let _ = quench_runtime::execute::set_property(
        receiver.clone(),
        "lastTotal",
        Value::Number(if pending_values.is_empty() { 0.0 } else { 3.0 }),
    );
    let _ = quench_runtime::execute::set_property(
        receiver.clone(),
        "lastChar",
        node_buffer(
            &pending_values
                .iter()
                .filter_map(|value| match value {
                    Value::Number(value) => Some(*value as u8),
                    _ => None,
                })
                .chain(std::iter::repeat(0))
                .take(4)
                .collect::<Vec<_>>(),
        ),
    );
    let _ = quench_runtime::execute::set_property(receiver.clone(), "_pending", pending);
    Ok(text_value)
}

fn string_decoder_end(receiver: Option<&Value>, arguments: &[Value]) -> Result<Value, VmError> {
    let receiver = receiver.ok_or(VmError::NotCallable)?;
    let prefix = if arguments.is_empty() {
        Value::String("".into())
    } else {
        string_decoder_write(Some(receiver), arguments)?
    };
    let pending = quench_runtime::execute::get_property_result(receiver, "_pending")
        .ok()
        .and_then(|value| array_values(&value).ok())
        .unwrap_or_default();
    let encoding = quench_runtime::execute::get_property_result(receiver, "encoding")
        .ok()
        .and_then(|value| match value {
            Value::String(value) => Some(value),
            _ => None,
        })
        .unwrap_or_default();
    let pending_bytes: Vec<u8> = pending
        .iter()
        .filter_map(|value| match value {
            Value::Number(value) => Some(*value as u8),
            _ => None,
        })
        .collect();
    let _ = quench_runtime::execute::set_property(
        receiver.clone(),
        "_pending",
        quench_runtime::host_api::array(Vec::new()),
    );
    let prefix_str = match &prefix {
        Value::String(value) => Some(value.as_str()),
        _ => None,
    };
    if encoding == "utf16le" || encoding == "ucs2" {
        let mut units: Vec<u16> = prefix_str
            .map(|value| value.encode_utf16().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut i = 0usize;
        while i + 1 < pending_bytes.len() {
            units.push(u16::from_le_bytes([pending_bytes[i], pending_bytes[i + 1]]));
            i += 2;
        }
        return Ok(if units.iter().any(|unit| (0xd800..=0xdfff).contains(unit)) {
            Value::StringUnits(Rc::new(units))
        } else {
            Value::String(String::from_utf16(&units).unwrap_or_default().into())
        });
    }
    let tail = if pending_bytes.is_empty() {
        String::new()
    } else if encoding == "base64" {
        sd_base64_encode(&pending_bytes, false)
    } else if encoding == "base64url" {
        sd_base64_encode(&pending_bytes, true)
    } else if encoding == "hex" {
        sd_hex_encode(&pending_bytes)
    } else {
        "�".into()
    };
    let prefix = prefix_str.unwrap_or_default();
    Ok(Value::String(format!("{prefix}{tail}").into()))
}


fn string_decoder_text(receiver: Option<&Value>, arguments: &[Value]) -> Result<Value, VmError> {
    let input = arguments.first().ok_or(VmError::NotCallable)?;
    let offset = arguments
        .get(1)
        .and_then(|value| match value {
            Value::Number(value) => Some((*value).max(0.0) as usize),
            _ => None,
        })
        .unwrap_or(0);
    let bytes = string_decoder_bytes(input)?;
    if offset >= bytes.len() {
        return Ok(Value::String("".into()));
    }
    string_decoder_write(receiver, &[node_buffer(&bytes[offset..])])
}
