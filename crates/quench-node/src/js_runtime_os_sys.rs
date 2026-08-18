fn os_arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        "x86" => "ia32",
        value => value,
    }
}

#[cfg(unix)]
fn os_uname_field(field: impl FnOnce(&libc::utsname) -> *const libc::c_char) -> Option<String> {
    let mut uts = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut uts) } != 0 {
        return None;
    }
    let pointer = field(&uts);
    if pointer.is_null() {
        return None;
    }
    Some(
        unsafe { std::ffi::CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(not(unix))]
fn os_uname_field(_: impl FnOnce(&()) -> *const u8) -> Option<String> {
    None
}

fn os_memory_bytes() -> f64 {
    #[cfg(unix)]
    {
        let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
        let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if pages > 0 && size > 0 {
            return pages as f64 * size as f64;
        }
    }
    1024.0 * 1024.0 * 1024.0
}

fn os_cpus() -> Result<Value, VmError> {
    let count = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .max(1);
    let model = os_cpu_model();
    let cpu = quench_runtime::host_api::object(vec![
        ("model".into(), Value::String(model.into())),
        ("speed".into(), Value::Number(0.0)),
        (
            "times".into(),
            quench_runtime::host_api::object(vec![
                ("user".into(), Value::Number(0.0)),
                ("nice".into(), Value::Number(0.0)),
                ("sys".into(), Value::Number(0.0)),
                ("idle".into(), Value::Number(0.0)),
                ("irq".into(), Value::Number(0.0)),
            ]),
        ),
    ]);
    Ok(quench_runtime::host_api::array(vec![cpu; count]))
}

fn os_cpu_model() -> String {
    std::env::consts::ARCH.to_string()
}
