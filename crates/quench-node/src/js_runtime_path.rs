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

    let mut start = if len >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
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

fn path_normalize(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let value = path_arg(arguments, 0)?;
    let separator = if win32 { '\\' } else { '/' };
    let value = if win32 {
        value.replace('/', "\\")
    } else {
        value.replace('\\', "/")
    };
    let absolute =
        value.starts_with(separator) || (win32 && value.len() > 2 && value.as_bytes()[1] == b':');
    let mut parts = Vec::new();
    for part in value.split(separator) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    let mut result = parts.join(&separator.to_string());
    if absolute && !(win32 && result.len() > 1 && result.as_bytes()[1] == b':') {
        result = format!("{separator}{result}");
    }
    if result.is_empty() {
        result = ".".into();
    }
    Ok(Value::String(result.into()))
}

fn path_parse(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let value = path_arg(arguments, 0)?;
    let separator = if win32 && value.starts_with('/') {
        '/'
    } else if win32 {
        '\\'
    } else {
        '/'
    };
    let normalized = if win32 && separator == '\\' {
        value.replace('/', "\\")
    } else {
        value.to_owned()
    };
    let root = if win32
        && normalized.len() >= 3
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[2] == b'\\'
    {
        &normalized[..3]
    } else if win32 && normalized.len() == 2 && normalized.as_bytes()[1] == b':' {
        &normalized[..2]
    } else if win32 && normalized.starts_with("\\\\") {
        let mut parts = normalized.split('\\').filter(|part| !part.is_empty());
        let server = parts.next().unwrap_or("");
        let share = parts.next().unwrap_or("");
        return path_parse_windows_with_root(&normalized, &format!("\\\\{server}\\{share}\\"));
    } else if win32 && (normalized.starts_with('\\') || normalized.starts_with('/')) {
        if separator == '/' {
            "/"
        } else {
            "\\"
        }
    } else if !win32 && value.starts_with('/') {
        "/"
    } else {
        ""
    };
    let trimmed = normalized.trim_end_matches(separator);
    let (dir, base) = if win32 && normalized.len() == 2 && normalized.as_bytes()[1] == b':' {
        (root, "")
    } else if win32
        && normalized.len() == 3
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[2] == b'\\'
    {
        (root, "")
    } else if trimmed.is_empty() && !root.is_empty() {
        (root, "")
    } else {
        trimmed
            .rsplit_once(separator)
            .map_or((root, trimmed), |(dir, base)| (dir, base))
    };
    let dir_with_extra_separator = if win32 {
        normalized.rsplit_once(separator).and_then(|(prefix, _)| {
            prefix
                .ends_with(separator)
                .then(|| format!("{dir}{separator}"))
        })
    } else {
        None
    };
    let dir = dir_with_extra_separator.as_deref().unwrap_or(dir);
    let (name, ext) = base
        .rfind('.')
        .filter(|index| *index > 0)
        .map_or((base, ""), |index| (&base[..index], &base[index..]));
    Ok(Value::object(vec![
        ("root".into(), Value::String(root.to_string().into())),
        ("dir".into(), Value::String(dir.to_string().into())),
        ("base".into(), Value::String(base.to_string().into())),
        ("ext".into(), Value::String(ext.to_string().into())),
        ("name".into(), Value::String(name.to_string().into())),
    ]))
}

fn path_parse_windows_with_root(value: &str, root: &str) -> Result<Value, VmError> {
    let trimmed = value.trim_end_matches('\\');
    let (dir, base) = trimmed
        .rsplit_once('\\')
        .map_or((root, trimmed), |(dir, base)| (dir, base));
    let (name, ext) = base
        .rfind('.')
        .filter(|index| *index > 0)
        .map_or((base, ""), |index| (&base[..index], &base[index..]));
    Ok(Value::object(vec![
        ("root".into(), Value::String(root.to_owned().into())),
        ("dir".into(), Value::String(dir.to_owned().into())),
        ("base".into(), Value::String(base.to_owned().into())),
        ("ext".into(), Value::String(ext.to_owned().into())),
        ("name".into(), Value::String(name.to_owned().into())),
    ]))
}

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
    let dir = {
        let value = string_prop("dir");
        if value.is_empty() {
            string_prop("root")
        } else {
            value
        }
    };
    let base = get("base")
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| {
            let name = string_prop("name");
            let ext = string_prop("ext");
            let ext = if ext.is_empty() || ext.starts_with('.') {
                ext
            } else {
                format!(".{ext}")
            };
            format!("{name}{ext}")
        });
    let separator = if win32 { '\\' } else { '/' };
    let output = if dir.is_empty() {
        base
    } else if win32 && dir.ends_with(':') {
        format!("{dir}{base}")
    } else {
        format!(
            "{}{}{}",
            dir.strip_suffix(separator).unwrap_or(dir.as_str()),
            separator,
            base
        )
    };
    Ok(Value::String(output.into()))
}

fn path_relative(arguments: &[Value]) -> Result<Value, VmError> {
    let from = path_arg(arguments, 0)?;
    let to = path_arg(arguments, 1)?;
    if from.contains('\\') || to.contains('\\') {
        let from = from.replace('/', "\\");
        let to = to.replace('/', "\\");
        let from = from
            .split('\\')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let to = to
            .split('\\')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let common = from
            .iter()
            .zip(&to)
            .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
            .count();
        let mut result = vec![".."; from.len().saturating_sub(common)];
        result.extend(to[common..].iter().copied());
        return Ok(Value::String(result.join("\\")));
    }
    let from = from
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    let to = to
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut result = vec![".."; from.len().saturating_sub(common)];
    result.extend(to[common..].iter().copied());
    Ok(Value::String(result.join("/")))
}

fn path_join(arguments: &[Value]) -> Result<Value, VmError> {
    let mut path = PathBuf::new();
    for argument in arguments {
        path.push(path_arg(std::slice::from_ref(argument), 0)?);
    }
    let joined = Value::String(path.to_string_lossy().into_owned().into());
    path_normalize(&[joined], false)
}

/// Mirrors Node's `path.extname`. `win32` splits on both separators and skips
/// a leading drive root; the posix variant splits on `/` only (a backslash is
/// an ordinary character) and still honors the drive-prefix quirk Node shares
/// across both platforms.
fn path_extname_core(input: &str, win32: bool) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len() as isize;
    let is_sep = |c: u8| c == b'/' || (win32 && c == b'\\');
    let start: isize = if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
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
    Ok(Value::Boolean(
        value.starts_with('/') || (value.len() > 2 && value.as_bytes()[1] == b':'),
    ))
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

fn path_resolve(arguments: &[Value], win32: bool) -> Result<Value, VmError> {
    let separator = if win32 { '\\' } else { '/' };
    let mut result = String::new();
    for argument in arguments {
        let value = path_arg(std::slice::from_ref(argument), 0)?;
        if win32 && value.len() > 2 && value.as_bytes()[1] == b':' {
            result = value.to_string();
            continue;
        }
        if value.starts_with(separator) {
            result = value.to_string();
        } else if result.is_empty() {
            result = value.to_string();
        } else {
            result = format!("{}{}{}", result.trim_end_matches(separator), separator, value);
        }
    }
    if !result.starts_with(separator) && !(win32 && result.len() > 2 && result.as_bytes()[1] == b':')
    {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .into_owned();
        result = format!("{}{}{}", cwd, separator, result);
    }
    // Normalize the resolved absolute path (collapse `.` / `..` / repeated separators).
    let normalized = if win32 {
        result.replace('/', "\\")
    } else {
        result.replace('\\', "/")
    };
    let absolute =
        normalized.starts_with(separator) || (win32 && normalized.len() > 2 && normalized.as_bytes()[1] == b':');
    let mut parts = Vec::new();
    for part in normalized.split(separator) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    let mut joined = parts.join(&separator.to_string());
    if absolute && !(win32 && joined.len() > 1 && joined.as_bytes()[1] == b':') {
        joined = format!("{separator}{joined}");
    }
    Ok(Value::String(joined.into()))
}

fn path_win_to_namespaced(arguments: &[Value]) -> Result<Value, VmError> {
    let Some(value) = arguments.first() else {
        return Ok(Value::Undefined);
    };
    let Value::String(value) = value else {
        return Ok(value.clone());
    };
    let value = value.replace('/', "\\");
    if value.starts_with("\\\\") {
        Ok(Value::String(format!(
            "\\\\?\\UNC\\{}\\",
            value.trim_start_matches("\\\\")
        )))
    } else if value.len() > 2 && value.as_bytes()[1] == b':' {
        Ok(Value::String(format!("\\\\?\\{}", value)))
    } else {
        Ok(Value::String(value))
    }
}
