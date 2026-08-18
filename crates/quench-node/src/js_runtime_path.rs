fn path_arg(arguments: &[Value], index: usize) -> Result<&str, VmError> {
    match arguments.get(index) {
        Some(Value::String(value)) => Ok(value),
        _ => Err(VmError::Thrown(fs_error(
            "ERR_INVALID_ARG_TYPE",
            "path must be a string",
        ))),
    }
}

fn path_value(arguments: &[Value], index: usize) -> Result<String, VmError> {
    match arguments.get(index) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(Value::Object(_)) => {
            let href =
                quench_runtime::execute::get_property_result(arguments.get(index).unwrap(), "href")
                    .ok()
                    .and_then(|value| match value {
                        Value::String(value) => Some(value),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        VmError::Thrown(fs_error(
                            "ERR_INVALID_ARG_TYPE",
                            "path must be a string or URL",
                        ))
                    })?;
            Ok(href.strip_prefix("file://").unwrap_or(&href).to_owned())
        }
        _ => Err(VmError::Thrown(fs_error(
            "ERR_INVALID_ARG_TYPE",
            "path must be a string or URL",
        ))),
    }
}

fn encode_file_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._/&=:;".contains(byte) {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn decode_file_url(value: &str) -> String {
    value
        .strip_prefix("file://")
        .unwrap_or(value)
        .replace("%20", " ")
        .replace("%23", "#")
        .replace("%5C", "\\")
        .replace("%5c", "\\")
}

/// Mirrors Node's `path.basename` for a separator/`win32` configuration:
///
/// - A leading drive root (`"C:"`) is skipped so it is not part of the result
///   and does not turn the separator after it into a trailing separator.
/// - The optional `suffix` is stripped only when it matches from the end of the
///   component and does not consume the entire component. When the suffix
///   equals the whole component or the whole path, Node returns the component
///   unchanged (or an empty string when `suffix === path`).
fn path_basename_core(input: &str, suffix: Option<&str>, win32: bool) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let is_sep = |c: u8| c == b'/' || (win32 && c == b'\\');

    let mut start = if win32 && len >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        2
    } else {
        0
    };

    let Some(suffix) = suffix.filter(|s| !s.is_empty() && s.len() <= len) else {
        let mut end: isize = -1;
        let mut matched_slash = true;
        let mut i = len;
        while i > start {
            i -= 1;
            if is_sep(bytes[i]) {
                if !matched_slash {
                    start = i + 1;
                    break;
                }
            } else if end == -1 {
                matched_slash = false;
                end = i as isize + 1;
            }
        }
        if end == -1 {
            return String::new();
        }
        return input[start..end as usize].to_string();
    };

    if suffix == input {
        return String::new();
    }
    let suffix_bytes = suffix.as_bytes();
    let mut ext_idx: isize = suffix.len() as isize - 1;
    let mut first_non_slash_end: isize = -1;
    let mut end: isize = -1;
    let mut matched_slash = true;
    let mut i = len;
    while i > start {
        i -= 1;
        if is_sep(bytes[i]) {
            if !matched_slash {
                start = i + 1;
                break;
            }
        } else {
            if first_non_slash_end == -1 {
                matched_slash = false;
                first_non_slash_end = i as isize + 1;
            }
            if ext_idx >= 0 {
                if bytes[i] == suffix_bytes[ext_idx as usize] {
                    ext_idx -= 1;
                    if ext_idx == -1 {
                        end = i as isize;
                    }
                } else {
                    ext_idx = -1;
                    end = first_non_slash_end;
                }
            }
        }
    }
    if start as isize == end {
        end = first_non_slash_end;
    } else if end == -1 {
        end = len as isize;
    }
    let end = end as usize;
    if end <= start {
        String::new()
    } else {
        input[start..end].to_string()
    }
}

fn path_win_basename(arguments: &[Value]) -> Result<Value, VmError> {
    let input = path_arg(arguments, 0)?;
    let suffix = match arguments.get(1) {
        None => None,
        Some(Value::String(suffix)) => Some(suffix.as_str()),
        Some(_) => {
            return Err(VmError::Thrown(fs_error(
                "ERR_INVALID_ARG_TYPE",
                "suffix must be a string",
            )))
        }
    };
    let value = path_basename_core(input, suffix, true);
    Ok(Value::String(value.into()))
}

fn is_windows_path_separator(code: u8) -> bool {
    code == b'/' || code == b'\\'
}

fn is_windows_device_root(code: u8) -> bool {
    code.is_ascii_alphabetic()
}

fn is_windows_reserved_name(input: &str, colon_index: usize) -> bool {
    let device_part = input[..colon_index].to_ascii_uppercase();
    matches!(
        device_part.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
            | "COM\u{b9}"
            | "COM\u{b2}"
            | "COM\u{b3}"
            | "LPT\u{b9}"
            | "LPT\u{b2}"
            | "LPT\u{b3}"
    )
}

/// Port of Node's `normalizeString`. Appends the segments described by
/// `path`, collapsing `.` segments and honouring `..` (only above the root
/// when `allow_above_root`).
fn normalize_string(
    path: &str,
    allow_above_root: bool,
    separator: char,
    is_sep: impl Fn(u8) -> bool,
) -> String {
    let bytes = path.as_bytes();
    let len = bytes.len();
    let mut res = String::new();
    let mut last_segment_length = 0usize;
    let mut last_slash: isize = -1;
    let mut dots: i8 = 0;
    let mut code: u8 = b'?';
    let mut i = 0usize;
    loop {
        if i > len {
            break;
        }
        if i < len {
            code = bytes[i];
        } else {
            if is_sep(code) {
                break;
            }
            code = b'/';
        }
        if is_sep(code) {
            if last_slash == i as isize - 1 || dots == 1 {
                // Consecutive separator or a single-dot segment: skip.
                last_slash = i as isize;
                dots = 0;
            } else if dots == 2 {
                let ends_with_dotdot = res.len() >= 2
                    && last_segment_length == 2
                    && res.as_bytes()[res.len() - 1] == b'.'
                    && res.as_bytes()[res.len() - 2] == b'.';
                if !ends_with_dotdot {
                    // Pop the previous segment (Node `continue`s so the `..`
                    // that resolved something is not re-added above the root).
                    if res.len() > 2 {
                        let last_slash_index =
                            res.len() as isize - last_segment_length as isize - 1;
                        if last_slash_index < 0 {
                            res.clear();
                            last_segment_length = 0;
                        } else {
                            res.truncate(last_slash_index as usize);
                            last_segment_length = match res.rfind(separator) {
                                Some(index) => res.len() - 1 - index,
                                None => res.len(),
                            };
                        }
                        last_slash = i as isize;
                        dots = 0;
                        i += 1;
                        continue;
                    } else if !res.is_empty() {
                        res.clear();
                        last_segment_length = 0;
                        last_slash = i as isize;
                        dots = 0;
                        i += 1;
                        continue;
                    }
                }
                if allow_above_root {
                    if res.is_empty() {
                        res.push_str("..");
                    } else {
                        res.push(separator);
                        res.push_str("..");
                    }
                    last_segment_length = 2;
                }
                last_slash = i as isize;
                dots = 0;
            } else {
                let segment_start = (last_slash + 1).max(0) as usize;
                let segment = &path[segment_start..i];
                if res.is_empty() {
                    res.push_str(segment);
                } else {
                    res.push(separator);
                    res.push_str(segment);
                }
                last_segment_length = (i as isize - last_slash - 1) as usize;
                last_slash = i as isize;
                dots = 0;
            }
        } else if code == b'.' && dots != -1 {
            dots += 1;
        } else {
            dots = -1;
        }
        i += 1;
    }
    res
}

fn posix_normalize(input: &str) -> String {
    if input.is_empty() {
        return ".".into();
    }
    let bytes = input.as_bytes();
    let is_absolute = bytes[0] == b'/';
    let trailing_separator = bytes[bytes.len() - 1] == b'/';
    let mut path = normalize_string(input, !is_absolute, '/', |c| c == b'/');
    if path.is_empty() {
        if is_absolute {
            return "/".into();
        }
        return if trailing_separator {
            "./".into()
        } else {
            ".".into()
        };
    }
    if trailing_separator {
        path.push('/');
    }
    if is_absolute {
        format!("/{path}")
    } else {
        path
    }
}

fn win32_normalize(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return ".".into();
    }
    let mut root_end = 0usize;
    let mut device: Option<String> = None;
    let mut is_absolute = false;
    let code = bytes[0];

    if len == 1 {
        return if code == b'/' { "\\".into() } else { input.to_string() };
    }
    if is_windows_path_separator(code) {
        is_absolute = true;
        if is_windows_path_separator(bytes[1]) {
            let mut j = 2usize;
            let mut last = 2usize;
            while j < len && !is_windows_path_separator(bytes[j]) {
                j += 1;
            }
            if j < len && j != last {
                let first_part = &input[last..j];
                last = j;
                while j < len && is_windows_path_separator(bytes[j]) {
                    j += 1;
                }
                if j < len && j != last {
                    last = j;
                    while j < len && !is_windows_path_separator(bytes[j]) {
                        j += 1;
                    }
                    if j == len || j != last {
                        if first_part == "." || first_part == "?" {
                            device = Some(format!("\\\\{first_part}"));
                            root_end = 4;
                            if let Some(colon_index) = input.find(':') {
                                let possible_device = &input[4..=colon_index];
                                if is_windows_reserved_name(possible_device, possible_device.len() - 1)
                                {
                                    device = Some(format!("\\\\?\\{possible_device}"));
                                    root_end = 4 + possible_device.len();
                                }
                            }
                        } else if j == len {
                            return format!(
                                "\\\\{first_part}\\{}\\",
                                &input[last..]
                            );
                        } else {
                            device = Some(format!("\\\\{first_part}\\{}", &input[last..j]));
                            root_end = j;
                        }
                    }
                }
            }
        } else {
            root_end = 1;
        }
    } else {
        let colon_index = input.find(':').unwrap_or(len);
        if colon_index > 0 {
            if is_windows_device_root(code) && colon_index == 1 {
                device = Some(input[..2].to_string());
                root_end = 2;
                if len > 2 && is_windows_path_separator(bytes[2]) {
                    is_absolute = true;
                    root_end = 3;
                }
            } else if colon_index < len && is_windows_reserved_name(input, colon_index) {
                device = Some(input[..=colon_index].to_string());
                root_end = colon_index + 1;
            }
        }
    }

    let mut tail = if root_end < len {
        normalize_string(&input[root_end..], !is_absolute, '\\', is_windows_path_separator)
    } else {
        String::new()
    };
    if tail.is_empty() && !is_absolute {
        tail.push('.');
    }
    if !tail.is_empty() && is_windows_path_separator(bytes[len - 1]) {
        tail.push('\\');
    }

    if !is_absolute
        && device.is_none()
        && input.contains(':')
        && tail.len() >= 2
        && is_windows_device_root(tail.as_bytes()[0])
        && tail.as_bytes()[1] == b':'
    {
        return format!(".\\{tail}");
    }
    if !is_absolute && device.is_none() && input.contains(':') {
        let mut index = input.find(':');
        while let Some(colon) = index {
            if colon == len - 1 || is_windows_path_separator(bytes[colon + 1]) {
                return format!(".\\{tail}");
            }
            index = if colon + 1 < len {
                input[colon + 1..].find(':').map(|c| colon + 1 + c)
            } else {
                None
            };
        }
    }

    let device_bound = input.find(':').unwrap_or_else(|| {
        // Node: StringPrototypeSlice(path, 0, -1) drops the final character when
        // no colon is present; e.g. "COM9." resolves its device part to "COM9".
        input.char_indices().next_back().map(|(i, _)| i).unwrap_or(0)
    });
    if device_bound < len && is_windows_reserved_name(input, device_bound) {
        return format!(".\\{}{tail}", device.as_deref().unwrap_or(""));
    }
    match device {
        None => {
            if is_absolute {
                format!("\\{tail}")
            } else {
                tail
            }
        }
        Some(device) => {
            if is_absolute {
                format!("{device}\\{tail}")
            } else {
                format!("{device}{tail}")
            }
        }
    }
}

fn path_normalize(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let value = path_arg(arguments, 0)?;
    let result = if win32 {
        win32_normalize(value)
    } else {
        posix_normalize(value)
    };
    Ok(Value::String(result.into()))
}

struct PathParts {
    root: String,
    dir: String,
    base: String,
    ext: String,
    name: String,
}

/// Port of Node's `path.parse` for both platforms. Extracts the directory,
/// basename, extension, and root from a path.
fn path_parse_core(input: &str, win32: bool) -> PathParts {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let is_sep = |c: u8| c == b'/' || (win32 && c == b'\\');

    let empty = PathParts {
        root: String::new(),
        dir: String::new(),
        base: String::new(),
        ext: String::new(),
        name: String::new(),
    };
    if len == 0 {
        return empty;
    }

    let mut body;
    if win32 {
        let code = bytes[0];
        let mut root_end = 0usize;
        if len == 1 {
            if is_sep(code) {
                return PathParts {
                    root: input.to_string(),
                    dir: input.to_string(),
                    base: String::new(),
                    ext: String::new(),
                    name: String::new(),
                };
            }
            return PathParts {
                root: String::new(),
                dir: String::new(),
                base: input.to_string(),
                ext: String::new(),
                name: input.to_string(),
            };
        }
        if is_sep(code) {
            root_end = 1;
            if is_sep(bytes[1]) {
                let mut j = 2usize;
                let mut last = 2usize;
                while j < len && !is_sep(bytes[j]) {
                    j += 1;
                }
                if j < len && j != last {
                    last = j;
                    while j < len && is_sep(bytes[j]) {
                        j += 1;
                    }
                    if j < len && j != last {
                        last = j;
                        while j < len && !is_sep(bytes[j]) {
                            j += 1;
                        }
                        if j == len {
                            root_end = j;
                        } else if j != last {
                            root_end = j + 1;
                        }
                    }
                }
            }
        } else if is_windows_device_root(code) && bytes[1] == b':' {
            if len <= 2 {
                let s = input.to_string();
                return PathParts {
                    root: s.clone(),
                    dir: s,
                    base: String::new(),
                    ext: String::new(),
                    name: String::new(),
                };
            }
            root_end = 2;
            if is_sep(bytes[2]) {
                if len == 3 {
                    let s = input.to_string();
                    return PathParts {
                        root: s.clone(),
                        dir: s,
                        base: String::new(),
                        ext: String::new(),
                        name: String::new(),
                    };
                }
                root_end = 3;
            }
        }
        body = parse_split_tail(input, root_end, is_sep, win32);
        if root_end > 0 {
            body.root = input[..root_end].to_string();
            if body.dir.is_empty() {
                body.dir = body.root.clone();
            }
        }
    } else {
        let is_absolute = bytes[0] == b'/';
        let start = if is_absolute { 1 } else { 0 };
        let mut parts = parse_split_tail(input, start, is_sep, win32);
        if is_absolute {
            parts.root = "/".into();
            if parts.dir.is_empty() {
                parts.dir = "/".into();
            }
        }
        body = parts;
    }
    body
}

/// Runs the shared "get non-dir info" scan and, for win32, computes `dir`.
fn parse_split_tail(input: &str, start: usize, is_sep: impl Fn(u8) -> bool, win32: bool) -> PathParts {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut start_dot: isize = -1;
    let mut start_part = start;
    let mut end: isize = -1;
    let mut matched_slash = true;
    let mut pre_dot_state = 0i8;
    let mut i = len;
    while i > start {
        i -= 1;
        let code = bytes[i];
        if is_sep(code) {
            if !matched_slash {
                start_part = i + 1;
                break;
            }
            continue;
        }
        if end == -1 {
            matched_slash = false;
            end = i as isize + 1;
        }
        if code == b'.' {
            if start_dot == -1 {
                start_dot = i as isize;
            } else if pre_dot_state != 1 {
                pre_dot_state = 1;
            }
        } else if start_dot != -1 {
            pre_dot_state = -1;
        }
    }

    let mut parts = PathParts {
        root: String::new(),
        dir: String::new(),
        base: String::new(),
        ext: String::new(),
        name: String::new(),
    };
    if end != -1 {
        let end_u = end as usize;
        let spliced_start = start_part;
        let dotless = start_dot == -1
            || pre_dot_state == 0
            || (pre_dot_state == 1 && start_dot == end - 1 && start_dot == start_part as isize + 1);
        if dotless {
            parts.base = input[spliced_start..end_u].to_string();
            parts.name = parts.base.clone();
        } else {
            parts.name = input[spliced_start..start_dot as usize].to_string();
            parts.base = input[spliced_start..end_u].to_string();
            parts.ext = input[start_dot as usize..end_u].to_string();
        }
        if win32 {
            if start_part > 0 && start_part != start {
                parts.dir = input[..start_part - 1].to_string();
            }
        } else if start_part > 0 {
            parts.dir = input[..start_part - 1].to_string();
        }
    } else if win32 && start > 0 {
        parts.dir = input[..start].to_string();
    }
    parts
}

fn path_parse(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let input = path_arg(arguments, 0)?;
    let parts = path_parse_core(input, win32);
    Ok(Value::object(vec![
        ("root".into(), Value::String(parts.root.into())),
        ("dir".into(), Value::String(parts.dir.into())),
        ("base".into(), Value::String(parts.base.into())),
        ("ext".into(), Value::String(parts.ext.into())),
        ("name".into(), Value::String(parts.name.into())),
    ]))
}

/// Port of Node's `_format(sep, pathObject)`: `dir` defaults to `root`, and
/// `base` defaults to `name + formatExt(ext)`.
fn path_format(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let Some(Value::Object(object)) = arguments.first() else {
        return Err(VmError::Thrown(fs_error(
            "ERR_INVALID_ARG_TYPE",
            "path object must be an object",
        )));
    };
    let get = |name| {
        quench_runtime::execute::get_property_result(&Value::Object(object.clone()), name).ok()
    };
    let string_prop = |name| {
        get(name)
            .and_then(|value| match value {
                Value::String(value) => Some(value.to_string()),
                _ => None,
            })
            .unwrap_or_default()
    };
    let root = string_prop("root");
    let dir = string_prop("dir");
    let dir: String = if dir.is_empty() { root.clone() } else { dir };
    let base = string_prop("base");
    let base = if base.is_empty() {
        let name = string_prop("name");
        let ext = string_prop("ext");
        let ext = if ext.is_empty() || ext.starts_with('.') {
            ext
        } else {
            format!(".{ext}")
        };
        format!("{name}{ext}")
    } else {
        base
    };
    let separator = if win32 { '\\' } else { '/' };
    let output = if dir.is_empty() {
        base
    } else if dir == root {
        format!("{dir}{base}")
    } else {
        format!("{dir}{separator}{base}")
    };
    Ok(Value::String(output.into()))
}

fn win32_relative_str(from_orig: &str, to_orig: &str) -> String {
    if from_orig == to_orig {
        return String::new();
    }
    let from = from_orig.to_lowercase();
    let to = to_orig.to_lowercase();
    if from == to {
        return String::new();
    }

    if from_orig.len() != from.len() || to_orig.len() != to.len() {
        // Case preserved in the originals: compare split components
        // case-insensitively.
        let mut from_split: Vec<&str> = from_orig.split('\\').collect();
        let mut to_split: Vec<&str> = to_orig.split('\\').collect();
        if from_split.last() == Some(&"") {
            from_split.pop();
        }
        if to_split.last() == Some(&"") {
            to_split.pop();
        }
        let from_len = from_split.len();
        let to_len = to_split.len();
        let length = from_len.min(to_len);
        let mut i = 0usize;
        while i < length {
            if from_split[i].to_lowercase() != to_split[i].to_lowercase() {
                break;
            }
            i += 1;
        }
        if i == 0 {
            return to_orig.to_string();
        } else if i == length {
            if to_len > length {
                return to_split[i..].join("\\");
            }
            if from_len > length {
                let mut s = "..\\".repeat(from_len - 1 - i);
                s.push_str("..");
                return s;
            }
            return String::new();
        }
        return format!("{}{}", "..\\".repeat(from_len - i), to_split[i..].join("\\"));
    }

    let from_bytes = from.as_bytes();
    let to_bytes = to.as_bytes();
    let mut from_start = 0usize;
    while from_start < from.len() && from_bytes[from_start] == b'\\' {
        from_start += 1;
    }
    let mut from_end = from.len();
    while from_end - 1 > from_start && from_bytes[from_end - 1] == b'\\' {
        from_end -= 1;
    }
    let from_len = from_end - from_start;
    let mut to_start = 0usize;
    while to_start < to.len() && to_bytes[to_start] == b'\\' {
        to_start += 1;
    }
    let mut to_end = to.len();
    while to_end - 1 > to_start && to_bytes[to_end - 1] == b'\\' {
        to_end -= 1;
    }
    let to_len = to_end - to_start;

    let length = from_len.min(to_len);
    let mut last_common_sep: isize = -1;
    let mut i = 0usize;
    while i < length {
        let from_code = from_bytes[from_start + i];
        if from_code != to_bytes[to_start + i] {
            break;
        }
        if from_code == b'\\' {
            last_common_sep = i as isize;
        }
        i += 1;
    }

    if i != length {
        if last_common_sep == -1 {
            return to_orig.to_string();
        }
    } else {
        if to_len > length {
            if to_bytes[to_start + i] == b'\\' {
                return to_orig[to_start + i + 1..to_end].to_string();
            }
            if i == 2 {
                return to_orig[to_start + i..to_end].to_string();
            }
        }
        if from_len > length {
            if from_bytes[from_start + i] == b'\\' {
                last_common_sep = i as isize;
            } else if i == 2 {
                last_common_sep = 3;
            }
        }
        if last_common_sep == -1 {
            last_common_sep = 0;
        }
    }

    let mut out = String::new();
    let start = (from_start as isize + last_common_sep + 1).max(0) as usize;
    let mut k = start;
    while k <= from_end {
        if k == from_end || from_bytes[k] == b'\\' {
            out.push_str(if out.is_empty() { ".." } else { "\\.." });
        }
        k += 1;
    }
    let to_common = (to_start as isize + last_common_sep).max(0) as usize;
    if !out.is_empty() {
        return format!("{out}{}", &to_orig[to_common..to_end]);
    }
    if to_bytes[to_common] == b'\\' {
        return to_orig[to_common + 1..to_end].to_string();
    }
    to_orig[to_common..to_end].to_string()
}

fn path_relative(arguments: &[Value]) -> Result<Value, VmError> {
    let from = path_arg(arguments, 0)?;
    let to = path_arg(arguments, 1)?;
    Ok(Value::String(posix_relative_str(from, to).into()))
}

fn path_win_relative(arguments: &[Value]) -> Result<Value, VmError> {
    let cwd = resolve_cwd();
    let args_from = &[path_arg(arguments, 0)?.to_string()];
    let from_orig = win32_resolve_str(&[args_from[0].as_str()], &cwd);
    let args_to = &[path_arg(arguments, 1)?.to_string()];
    let to_orig = win32_resolve_str(&[args_to[0].as_str()], &cwd);
    Ok(Value::String(win32_relative_str(&from_orig, &to_orig).into()))
}

fn join_path_args(arguments: &[Value]) -> Result<Vec<&str>, VmError> {
    let mut parts = Vec::new();
    for argument in arguments {
        let part = path_arg(std::slice::from_ref(argument), 0)?;
        if !part.is_empty() {
            parts.push(part);
        }
    }
    Ok(parts)
}

fn host_cwd() -> String {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".into())
}

/// `path.resolve` calls `process.cwd()`, which the test suite overrides
/// (e.g. to `''` to exercise the failure fallback). Prefer the JS function so
/// the mock is honoured, falling back to the host directory.
fn resolve_cwd() -> String {
    let js = NODE_PROCESS_MODULE.with(|current| current.borrow().clone()).and_then(
        |process| {
            let cwd_fn = quench_runtime::execute::get_property_result(&process, "cwd").ok()?;
            match quench_runtime::execute::call(&cwd_fn, &process, &[]) {
                Ok(Value::String(value)) => Some(value.to_string()),
                _ => None,
            }
        },
    );
    js.unwrap_or_else(host_cwd)
}

fn posix_resolve_str(args: &[&str]) -> String {
    if args.is_empty() || (args.len() == 1 && (args[0].is_empty() || args[0] == ".")) {
        let cwd = resolve_cwd();
        if cwd.starts_with('/') {
            return cwd;
        }
    }
    let mut resolved = String::new();
    let mut absolute = false;
    for name in args.iter().rev() {
        if name.is_empty() {
            continue;
        }
        resolved = format!("{name}/{resolved}");
        absolute = name.starts_with('/');
        if absolute {
            break;
        }
    }
    if !absolute {
        let cwd = resolve_cwd();
        resolved = format!("{cwd}/{resolved}");
        absolute = cwd.starts_with('/');
    }
    let mut normalized = normalize_string(&resolved, !absolute, '/', |c| c == b'/');
    if normalized.is_empty() && absolute {
        normalized.clear();
    }
    if absolute {
        format!("/{normalized}")
    } else if normalized.is_empty() {
        ".".into()
    } else {
        normalized
    }
}

fn posix_relative_str(from: &str, to: &str) -> String {
    if from == to {
        return String::new();
    }
    let from = posix_resolve_str(&[from]);
    let to = posix_resolve_str(&[to]);
    if from == to {
        return String::new();
    }
    let from_bytes = from.as_bytes();
    let to_bytes = to.as_bytes();
    let from_start = 1usize;
    let from_end = from.len();
    let from_len = from_end - from_start;
    let to_start = 1usize;
    let to_len = to.len() - to_start;

    let length = from_len.min(to_len);
    let mut last_common_sep: isize = -1;
    let mut i = 0usize;
    while i < length {
        let from_code = from_bytes[from_start + i];
        if from_code != to_bytes[to_start + i] {
            break;
        }
        if from_code == b'/' {
            last_common_sep = i as isize;
        }
        i += 1;
    }
    if i == length {
        if to_len > length {
            if to_bytes[to_start + i] == b'/' {
                return to[to_start + i + 1..].to_string();
            }
            if i == 0 {
                return to[to_start + i..].to_string();
            }
        } else if from_len > length {
            if from_bytes[from_start + i] == b'/' {
                last_common_sep = i as isize;
            } else if i == 0 {
                last_common_sep = 0;
            }
        }
    }

    let mut out = String::new();
    let start = from_start as isize + last_common_sep + 1;
    let start = start.max(0) as usize;
    let mut k = start;
    while k <= from_end {
        if k == from_end || from_bytes[k] == b'/' {
            out.push_str(if out.is_empty() { ".." } else { "/.." });
        }
        k += 1;
    }
    let to_common = (to_start as isize + last_common_sep).max(0) as usize;
    out.push_str(&to[to_common..]);
    out
}

fn path_join(arguments: &[Value]) -> Result<Value, VmError> {
    let parts = join_path_args(arguments)?;
    if parts.is_empty() {
        return Ok(Value::String(".".into()));
    }
    Ok(Value::String(posix_normalize(&parts.join("/")).into()))
}

fn path_win_join(arguments: &[Value]) -> Result<Value, VmError> {
    let parts = join_path_args(arguments)?;
    if parts.is_empty() {
        return Ok(Value::String(".".into()));
    }
    let first_part = parts[0];
    let mut joined = parts.join("\\");
    let first_bytes = first_part.as_bytes();

    // Make sure the joined path does not start with two slashes, unless the
    // first part clearly was intended as a UNC server name.
    let mut needs_replace = true;
    let mut slash_count = 0usize;
    if is_windows_path_separator(first_bytes[0]) {
        slash_count += 1;
        let first_len = first_bytes.len();
        if first_len > 1 && is_windows_path_separator(first_bytes[1]) {
            slash_count += 1;
            if first_len > 2 {
                if is_windows_path_separator(first_bytes[2]) {
                    slash_count += 1;
                } else {
                    needs_replace = false;
                }
            }
        }
    }
    if needs_replace {
        let joined_bytes = joined.as_bytes();
        while slash_count < joined_bytes.len()
            && is_windows_path_separator(joined_bytes[slash_count])
        {
            slash_count += 1;
        }
        if slash_count >= 2 {
            joined = format!("\\{}", &joined[slash_count..]);
        }
    }

    // Preserve a path verbatim when any component names a reserved device.
    let reserved = joined.split('\\').any(|part| {
        !part.is_empty() && part.find(':').is_some_and(|ci| is_windows_reserved_name(part, ci))
    });
    if reserved {
        return Ok(Value::String(joined.replace('/', "\\").into()));
    }
    Ok(Value::String(win32_normalize(&joined).into()))
}

/// Mirrors Node's `path.extname`. `win32` splits on both separators and skips
/// a leading drive root; the posix variant splits on `/` only (a backslash is
/// an ordinary character) and still honors the drive-prefix quirk Node shares
/// across both platforms.
fn path_extname_core(input: &str, win32: bool) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len() as isize;
    let is_sep = |c: u8| c == b'/' || (win32 && c == b'\\');
    let start: isize = if win32
        && bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
    {
        2
    } else {
        0
    };
    let mut start_part = start;
    let mut start_dot: isize = -1;
    let mut end: isize = -1;
    let mut matched_slash = true;
    let mut pre_dot_state = 0i8;
    let mut i = len;
    while i > start {
        i -= 1;
        if is_sep(bytes[i as usize]) {
            if !matched_slash {
                start_part = i + 1;
                break;
            }
            continue;
        }
        if end == -1 {
            matched_slash = false;
            end = i + 1;
        }
        if bytes[i as usize] == b'.' {
            if start_dot == -1 {
                start_dot = i;
            } else if pre_dot_state != 1 {
                pre_dot_state = 1;
            }
        } else if start_dot != -1 {
            pre_dot_state = -1;
        }
    }
    if start_dot == -1
        || end == -1
        || pre_dot_state == 0
        || (pre_dot_state == 1 && start_dot == end - 1 && start_dot == start_part + 1)
    {
        return String::new();
    }
    let start_dot = start_dot as usize;
    let end = end as usize;
    if start_dot >= end || end > bytes.len() {
        return String::new();
    }
    input[start_dot..end].to_string()
}

fn path_extname(arguments: &[Value]) -> Result<Value, VmError> {
    let input = path_arg(arguments, 0)?;
    Ok(Value::String(path_extname_core(input, false).into()))
}

fn path_win_extname(arguments: &[Value]) -> Result<Value, VmError> {
    let input = path_arg(arguments, 0)?;
    Ok(Value::String(path_extname_core(input, true).into()))
}

/// Mirrors Node's `path.dirname`. The posix variant splits on `/` only (with
/// the `//`-root rule); the win32 variant also recognizes a drive root and a
/// UNC share as the directory root.
fn path_dirname_core(input: &str, win32: bool) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let is_sep = |c: u8| c == b'/' || (win32 && c == b'\\');
    let is_device_root = |c: u8| c.is_ascii_alphabetic();

    if len == 0 {
        return ".".into();
    }
    if len == 1 {
        return if is_sep(bytes[0]) {
            input.to_string()
        } else {
            ".".into()
        };
    }

    if !win32 {
        let has_root = bytes[0] == b'/';
        let mut end: isize = -1;
        let mut matched_slash = true;
        let mut i = len as isize;
        while i > 1 {
            i -= 1;
            if bytes[i as usize] == b'/' {
                if !matched_slash {
                    end = i;
                    break;
                }
            } else {
                matched_slash = false;
            }
        }
        if end == -1 {
            return if has_root { "/".into() } else { ".".into() };
        }
        if has_root && end == 1 {
            return "//".into();
        }
        return input[..end as usize].to_string();
    }

    // win32: locate the directory root (UNC share or device drive). The scan
    // begins just past the root so separators inside it are not treated as
    // component separators.
    let mut root_end: isize = -1;
    if is_sep(bytes[0]) {
        root_end = 1;
        if is_sep(bytes[1]) {
            // Possible UNC root: \\<server>\<share>\
            let (mut j, mut last) = (2usize, 2usize);
            while j < len && !is_sep(bytes[j]) {
                j += 1;
            }
            if j < len && j != last {
                last = j;
                while j < len && is_sep(bytes[j]) {
                    j += 1;
                }
                if j < len && j != last {
                    last = j;
                    while j < len && !is_sep(bytes[j]) {
                        j += 1;
                    }
                    if j == len {
                        return input.to_string();
                    }
                    if j != last {
                        root_end = j as isize + 1;
                    }
                }
            }
        }
    } else if is_device_root(bytes[0]) && bytes[1] == b':' {
        root_end = if len > 2 && is_sep(bytes[2]) { 3 } else { 2 };
    }
    let offset = if root_end == -1 { 0 } else { root_end };

    let mut end: isize = -1;
    let mut matched_slash = true;
    let mut i = len as isize;
    while i > offset {
        i -= 1;
        if is_sep(bytes[i as usize]) {
            if !matched_slash {
                end = i;
                break;
            }
        } else {
            matched_slash = false;
        }
    }
    if end == -1 {
        if root_end == -1 {
            return ".".into();
        }
        end = root_end;
    }
    let mut end = end as usize;
    if end > len {
        end = len;
    }
    input[..end].to_string()
}

fn path_dirname(arguments: &[Value]) -> Result<Value, VmError> {
    let input = path_arg(arguments, 0)?;
    Ok(Value::String(path_dirname_core(input, false).into()))
}

fn path_win_dirname(arguments: &[Value]) -> Result<Value, VmError> {
    let input = path_arg(arguments, 0)?;
    Ok(Value::String(path_dirname_core(input, true).into()))
}

fn path_is_absolute(arguments: &[Value]) -> Result<Value, VmError> {
    let value = path_arg(arguments, 0)?;
    Ok(Value::Boolean(value.starts_with('/')))
}

fn path_is_absolute_win(arguments: &[Value]) -> Result<Value, VmError> {
    let value = path_arg(arguments, 0)?;
    Ok(Value::Boolean(
        value.starts_with(['/', '\\'])
            || (value.len() > 2
                && value.as_bytes()[1] == b':'
                && matches!(value.as_bytes()[2], b'/' | b'\\')),
    ))
}

fn path_matches_glob(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let value = path_arg(arguments, 0)?;
    let pattern = path_arg(arguments, 1)?;
    let value = if win32 {
        value.replace('\\', "/")
    } else {
        value.to_owned()
    };
    let pattern = if win32 {
        pattern.replace('\\', "/")
    } else {
        pattern.to_owned()
    };
    let matched = if let Some(prefix) = pattern.strip_suffix("/**") {
        value == prefix || value.starts_with(&format!("{prefix}/"))
    } else if let Some(suffix) = pattern.strip_prefix("*.") {
        value.ends_with(&format!(".{suffix}"))
    } else {
        value == pattern
    };
    Ok(Value::Boolean(matched))
}

/// Port of Node's `win32.resolve`. Device and UNC roots are matched on each
/// argument from the right; drive-relative inputs resolve against the
/// process cwd (no drive-specific cwd env exists on this host, matching
/// Node's fallback path).
fn win32_resolve_str(args: &[&str], cwd: &str) -> String {
    if args.is_empty() || (args.len() == 1 && (args[0].is_empty() || args[0] == ".")) {
        if cwd.starts_with(['/', '\\']) {
            return cwd.replace('/', "\\");
        }
    }

    let mut resolved_device = String::new();
    let mut resolved_tail = String::new();
    let mut resolved_absolute = false;

    let mut i: isize = args.len() as isize - 1;
    loop {
        let path_str: String;
        if i >= 0 {
            let value = args[i as usize];
            path_str = value.to_string();
            if path_str.is_empty() {
                i -= 1;
                continue;
            }
        } else {
            if resolved_device.is_empty() {
                path_str = cwd.to_string();
            } else {
                path_str = cwd.to_string();
            }
        }

        let bytes = path_str.as_bytes();
        let len = bytes.len();
        let mut root_end = 0usize;
        let mut device = String::new();
        let mut is_absolute = false;
        let code = bytes[0];

        if len == 1 {
            if is_windows_path_separator(code) {
                root_end = 1;
                is_absolute = true;
            }
        } else if is_windows_path_separator(code) {
            is_absolute = true;
            if is_windows_path_separator(bytes[1]) {
                let mut j = 2usize;
                let mut last = 2usize;
                while j < len && !is_windows_path_separator(bytes[j]) {
                    j += 1;
                }
                if j < len && j != last {
                    let first_part = &path_str[last..j];
                    last = j;
                    while j < len && is_windows_path_separator(bytes[j]) {
                        j += 1;
                    }
                    if j < len && j != last {
                        last = j;
                        while j < len && !is_windows_path_separator(bytes[j]) {
                            j += 1;
                        }
                        if j == len || j != last {
                            if first_part != "." && first_part != "?" {
                                device = format!("\\\\{first_part}\\{}", &path_str[last..j]);
                                root_end = j;
                            } else {
                                device = format!("\\\\{first_part}");
                                root_end = 4;
                            }
                        }
                    }
                }
            } else {
                root_end = 1;
            }
        } else if is_windows_device_root(code) && bytes[1] == b':' {
            device = path_str[..2].to_string();
            root_end = 2;
            if len > 2 && is_windows_path_separator(bytes[2]) {
                is_absolute = true;
                root_end = 3;
            }
        }

        if !device.is_empty() {
            if !resolved_device.is_empty() {
                if !device.eq_ignore_ascii_case(&resolved_device) {
                    i -= 1;
                    continue;
                }
            } else {
                resolved_device = device;
            }
        }

        if resolved_absolute {
            if !resolved_device.is_empty() {
                break;
            }
        } else {
            resolved_tail = format!("{}\\{resolved_tail}", &path_str[root_end..]);
            resolved_absolute = is_absolute;
            if is_absolute && !resolved_device.is_empty() {
                break;
            }
        }
        if i < 0 {
            break;
        }
        i -= 1;
    }

    let mut tail = normalize_string(&resolved_tail, !resolved_absolute, '\\', is_windows_path_separator);
    if tail.is_empty() && resolved_absolute {
        tail.clear();
    }
    if resolved_absolute {
        format!("{resolved_device}\\{tail}")
    } else {
        let combined = format!("{resolved_device}{tail}");
        if combined.is_empty() {
            ".".to_string()
        } else {
            combined
        }
    }
}

fn path_resolve(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let mut names = Vec::new();
    for argument in arguments {
        names.push(path_arg(std::slice::from_ref(argument), 0)?.to_string());
    }
    let cwd = resolve_cwd();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let result = if win32 {
        win32_resolve_str(&names, &cwd)
    } else {
        posix_resolve_str(&names)
    };
    Ok(Value::String(result.into()))
}

/// Node's default and `path.posix` `toNamespacedPath` are the identity
/// function: the argument is returned unchanged, whatever its type. Only the
/// win32 variant rewrites the path.
fn path_to_namespaced(arguments: &[Value]) -> Result<Value, VmError> {
    Ok(arguments.first().cloned().unwrap_or(Value::Undefined))
}

/// Port of Node's `path.win32.toNamespacedPath`: resolve the path, then fold
/// it into a `\\?\`-prefixed long path when it is a drive-rooted or UNC path.
fn path_win_to_namespaced(arguments: &[Value]) -> Result<Value, VmError> {
    let Some(value) = arguments.first() else {
        return Ok(Value::Undefined);
    };
    let Value::String(path) = value else {
        return Ok(value.clone());
    };
    if path.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let resolved = win32_resolve_str(&[path.as_str()], &resolve_cwd());
    if resolved.len() <= 2 {
        return Ok(Value::String(path.clone()));
    }
    let bytes = resolved.as_bytes();
    if bytes[0] == b'\\' {
        if bytes.get(1) == Some(&b'\\') {
            let code2 = bytes.get(2).copied();
            if code2 != Some(b'?') && code2 != Some(b'.') {
                return Ok(Value::String(format!("\\\\?\\UNC\\{}", &resolved[2..])));
            }
        }
    } else if is_windows_device_root(bytes[0])
        && bytes.get(1) == Some(&b':')
        && bytes.get(2) == Some(&b'\\')
    {
        return Ok(Value::String(format!("\\\\?\\{resolved}")));
    }
    Ok(Value::String(resolved))
}
