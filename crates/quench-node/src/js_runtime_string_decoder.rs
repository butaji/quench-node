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

/// Raw bytes behind any Buffer/TypedArray/DataView value (all element kinds).
fn array_view_bytes(value: &Value) -> Option<Vec<u8>> {
    macro_rules! bytes_of {
        ($view:expr, $offset:expr, $len:expr) => {
            $view.buffer.bytes.borrow()[$offset..$offset + $len].to_vec()
        };
    }
    match value {
        Value::Uint8Array(v) => Some(bytes_of!(v, v.byte_offset, v.length)),
        Value::Uint8ClampedArray(v) => Some(bytes_of!(v, v.byte_offset, v.length)),
        Value::Int8Array(v) => Some(bytes_of!(v, v.byte_offset, v.length)),
        Value::Int16Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 2)),
        Value::Uint16Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 2)),
        Value::Int32Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 4)),
        Value::Uint32Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 4)),
        Value::Float32Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 4)),
        Value::Float64Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 8)),
        Value::BigInt64Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 8)),
        Value::BigUint64Array(v) => Some(bytes_of!(v, v.byte_offset, v.length * 8)),
        Value::DataView(v) => Some(bytes_of!(v, v.byte_offset, v.byte_length)),
        _ => None,
    }
}

fn string_decoder_bytes(value: &Value) -> Result<Vec<u8>, VmError> {
    let bytes = match array_view_bytes(value) {
        Some(bytes) => bytes,
        None => {
            let received = match value {
                Value::Null => "null".to_string(),
                Value::Undefined => "undefined".to_string(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
                Value::String(s) => format!("\"{s}\""),
                _ => "type object".to_string(),
            };
            return Err(string_decoder_invalid_arg(received));
        }
    };
    Ok(bytes)
}

/// Node-style `ERR_INVALID_ARG_TYPE` TypeError for the `buf` argument.
fn string_decoder_invalid_arg(received: String) -> VmError {
    let message =
        "The \"buf\" argument must be an instance of Buffer, TypedArray, or DataView. Received ";
    VmError::Thrown(quench_runtime::host_api::object(vec![
        ("code".into(), Value::String("ERR_INVALID_ARG_TYPE".into())),
        ("name".into(), Value::String("TypeError".into())),
        ("message".into(), Value::String(format!("{message}{received}").into())),
    ]))
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

/// Node-compatible UTF-8 streaming decoder. Returns the decoded text and any
/// incomplete trailing byte sequence to carry into the next write. Invalid
/// byte sequences are replaced with U+FFFD, advancing byte-by-byte so that a
/// bad lead byte does not swallow a following valid continuation.
fn decode_utf8_stream(bytes: &[u8]) -> (String, Vec<u8>) {
    fn char_len(lead: u8) -> usize {
        match lead {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            _ => 0,
        }
    }
    fn payload(lead: u8) -> u8 {
        match lead {
            0xc0..=0xdf => lead & 0x1f,
            0xe0..=0xef => lead & 0x0f,
            0xf0..=0xf7 => lead & 0x07,
            _ => 0,
        }
    }
    let mut out = String::new();
    let mut i = 0usize;
    let len = bytes.len();
    while i < len {
        let lead = bytes[i];
        let clen = char_len(lead);
        if clen == 1 {
            out.push(lead as char);
            i += 1;
            continue;
        }
        if clen == 0 {
            out.push('�');
            i += 1;
            continue;
        }
        // Count contiguous valid continuation bytes after the lead.
        let mut has = 0usize;
        while i + 1 + has < len && bytes[i + 1 + has] & 0xc0 == 0x80 {
            has += 1;
        }
        if has < clen - 1 {
            if i + 1 + has >= len {
                // Buffer ran out mid-sequence: carry to the next write.
                return (out, bytes[i..].to_vec());
            }
            // A non-continuation byte interrupted: the lead plus its collected
            // continuation bytes form one invalid unit (Node/V8 rule).
            out.push('�');
            i += 1 + has;
            continue;
        }
        let mut cp = payload(lead) as u32;
        for k in 1..clen {
            cp = (cp << 6) | (bytes[i + k] as u32 & 0x3f);
        }
        if let Some(ch) = char::from_u32(cp) {
            out.push(ch);
            i += clen;
        } else {
            // Overlong / surrogate / out-of-range sequence.
            out.push('�');
            i += 1;
        }
    }
    (out, Vec::new())
}

fn string_decoder_write(receiver: Option<&Value>, arguments: &[Value]) -> Result<Value, VmError> {
    let receiver = receiver.ok_or(VmError::NotCallable)?;
    if quench_runtime::execute::get_property_result(receiver, "_pending").is_err() {
        return Err(VmError::Thrown(quench_runtime::host_api::object(vec![
            ("code".into(), Value::String("ERR_INVALID_THIS".into())),
            ("name".into(), Value::String("TypeError".into())),
        ])));
    }
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
        let (text, pending) = decode_utf8_stream(&bytes);
        (Value::String(text.into()), pending)
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
