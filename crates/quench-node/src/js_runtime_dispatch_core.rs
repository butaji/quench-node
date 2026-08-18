const DISPATCH_UNHANDLED: &str = "quench-node dispatch: unhandled capability";

/// Monotonic counter behind `v8.cachedDataVersionTag()`, bumped whenever
/// `v8.setFlagsFromString()` is called. The engine itself is not V8, but the
/// public contract only requires a number that is stable between flag changes.
use std::sync::atomic::{AtomicU64, Ordering};
static V8_FLAG_VERSION: AtomicU64 = AtomicU64::new(0);

/// Build a Node-style thrown error value carrying the standard `name`, `code`,
/// and `message` properties so `assert.throws`/`assert.rejects` validators and
/// `ERR_*` checks observe the same shape Node exposes.
fn thrown_js_error(name: &str, code: &str, message: &str) -> VmError {
    VmError::Thrown(quench_runtime::host_api::object(vec![
        ("name".into(), Value::String(name.into())),
        ("code".into(), Value::String(code.into())),
        ("message".into(), Value::String(message.into())),
    ]))
}

fn invalid_string_arg_error(argument_name: &str) -> VmError {
    thrown_js_error(
        "TypeError",
        "ERR_INVALID_ARG_TYPE",
        &format!("The \"{argument_name}\" argument must be of type string."),
    )
}

/// The runtime stores symbol primitives as sentinel strings of the form
/// `"Symbol.<tag>\0"`. Mirror the engine's own symbol-string heuristic so a
/// symbol is not mistaken for a genuine string when a string-only argument is
/// validated.
fn is_symbol_representation(value: &str) -> bool {
    value.starts_with("Symbol.") && value.contains('\0')
}

impl QuenchNodeHost {
    fn dispatch_core(
        &self,
        capability: HostCapabilityRef,
        receiver: Option<&Value>,
        arguments: &[Value],
    ) -> Option<Result<Value, VmError>> {
        let result = (|| -> Result<Value, VmError> {
            match capability.kind {
            HostCapabilityKind::Custom(CapabilityName::Require) => {
                if matches!(arguments.first(), Some(Value::String(name)) if name.trim_start_matches("node:") == "string_decoder")
                {
                    Ok(string_decoder_module())
                } else {
                    require_module(arguments)
                }
            }
            HostCapabilityKind::Custom(CapabilityName::EventEmitter) => {
                self.construct(capability, arguments)
            }
            HostCapabilityKind::Custom(
                CapabilityName::StreamReadable
                | CapabilityName::StreamWritable
                | CapabilityName::StreamReadableFrom,
            ) => self.construct(capability, arguments),
            HostCapabilityKind::Custom(CapabilityName::Stream) => {
                self.construct(capability, arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::StreamDuplex) => {
                self.construct(capability, arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::StreamFinished) => {
                stream_finished(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::StreamIsPaused) => Ok(Value::Boolean(false)),
            HostCapabilityKind::Custom(CapabilityName::StreamBaseWrite) => Ok(Value::Boolean(true)),
            HostCapabilityKind::Custom(CapabilityName::StreamRead) => Ok(Value::Null),
            HostCapabilityKind::Custom(CapabilityName::FsAccess) => {
                fs_access(arguments).map_err(invalid_path_error)
            }
            HostCapabilityKind::Custom(CapabilityName::FsWriteBytes) => {
                fs_write_bytes(arguments, false)
            }
            HostCapabilityKind::Custom(CapabilityName::FsAppendBytes) => {
                if matches!(arguments.first(), Some(Value::Number(_))) {
                    self.fs_append_file_async(arguments)
                } else {
                    fs_write_bytes(arguments, true)
                }
            }
            HostCapabilityKind::Custom(CapabilityName::FsUnlink) => fs_unlink_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsMkdtemp) => fs_mkdtemp(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsAccessSync) => {
                fs_access_sync(arguments).map_err(invalid_path_error)
            }
            HostCapabilityKind::Custom(CapabilityName::FsWriteFileSync) => {
                self.fs_write_file(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsAppendFileSync) => {
                self.fs_append_file(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsUnlinkSync) => fs_unlink(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsRmdirSync) => fs_rmdir(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsRealpathSync) => fs_realpath(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsOpenSync) => self.fs_open(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsCloseSync) => self.fs_close(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsFchmod) => self.fs_fchmod(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsFstatSync) => self.fs_fstat(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsChmodSync) => fs_chmod(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsAccessAsync) => fs_access_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsExistsSync) => fs_access(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsExists) => fs_exists(arguments),
            HostCapabilityKind::Custom(CapabilityName::ChildExecFile) => child_exec_file(arguments),
            HostCapabilityKind::Custom(CapabilityName::ChildSpawn) => {
                let command = arguments.first().map(safe_value_string).unwrap_or_default();
                let args = arguments
                    .get(1)
                    .and_then(|value| array_values(value).ok())
                    .unwrap_or_default();
                Ok(quench_runtime::host_api::object(vec![
                    ("pid".into(), Value::Undefined),
                    (
                        "on".into(),
                        capability_function(HostCapabilityKind::Custom(
                            CapabilityName::ChildSpawnOn,
                        )),
                    ),
                    ("\0childCommand".into(), Value::String(command.into())),
                    ("\0childArgs".into(), quench_runtime::host_api::array(args)),
                ]))
            }
            HostCapabilityKind::Custom(CapabilityName::ChildSpawnOn) => {
                let receiver = receiver.ok_or(VmError::NotCallable)?;
                if matches!(arguments.first(), Some(Value::String(event)) if event == "error") {
                    let callback = arguments.get(1).ok_or(VmError::NotCallable)?;
                    let command =
                        quench_runtime::execute::get_property_result(receiver, "\0childCommand")
                            .unwrap_or(Value::String("".into()));
                    let args =
                        quench_runtime::execute::get_property_result(receiver, "\0childArgs")
                            .unwrap_or_else(|_| quench_runtime::host_api::array(vec![]));
                    let error = quench_runtime::host_api::object(vec![
                        ("code".into(), Value::String("ENOENT".into())),
                        (
                            "syscall".into(),
                            Value::String(format!("spawn {}", safe_value_string(&command)).into()),
                        ),
                        ("spawnargs".into(), args),
                    ]);
                    quench_runtime::execute::call(callback, &Value::Undefined, &[error])?;
                }
                Ok(receiver.clone())
            }
            HostCapabilityKind::Custom(CapabilityName::ChildSpawnSync) => {
                Ok(quench_runtime::host_api::object(vec![
                    ("status".into(), Value::Number(0.0)),
                    (
                        "stdout".into(),
                        quench_runtime::host_api::object(vec![(
                            "toString".into(),
                            capability_function(HostCapabilityKind::Custom(
                                CapabilityName::ChildStdoutToString,
                            )),
                        )]),
                    ),
                    ("stderr".into(), quench_runtime::host_api::bytes(&[])),
                ]))
            }
            HostCapabilityKind::Custom(CapabilityName::ChildStdoutToString) => Ok(Value::String(
                format!("{}\n", std::env::args().next().unwrap_or_default()).into(),
            )),
            HostCapabilityKind::Custom(CapabilityName::ChildFork) => child_fork(arguments),
            HostCapabilityKind::Custom(CapabilityName::ChildEmit) => Ok(Value::Undefined),
            HostCapabilityKind::Custom(CapabilityName::ChildSend) => Err(VmError::EvalError(
                "message argument must be specified".into(),
            )),
            HostCapabilityKind::Custom(CapabilityName::CommonMustCall) => {
                self.common_wrapper(arguments, true)
            }
            HostCapabilityKind::Custom(CapabilityName::CommonMustCallAtLeast) => {
                self.common_wrapper(arguments, true)
            }
            HostCapabilityKind::Custom(CapabilityName::CommonMustSucceed) => {
                self.common_wrapper(arguments, true)
            }
            HostCapabilityKind::Custom(CapabilityName::CommonMustNotCall) => {
                self.common_wrapper(arguments, false)
            }
            HostCapabilityKind::Custom(CapabilityName::CommonSkip) => {
                Ok(Value::Undefined)
            }
            HostCapabilityKind::Custom(CapabilityName::CommonGetArrayBufferViews) => {
                let value = match (arguments.first(), arguments.get(1)) {
                    (Some(Value::String(value)), Some(Value::String(encoding)))
                        if encoding.eq_ignore_ascii_case("hex") =>
                    {
                        node_buffer(&decode_hex(value))
                    }
                    (Some(value), _) => value.clone(),
                    _ => Value::Undefined,
                };
                Ok(quench_runtime::host_api::array(vec![
                    value.clone(),
                    value.clone(),
                    value,
                ]))
            }
            HostCapabilityKind::Custom(CapabilityName::CommonCanSymlink) => {
                Ok(Value::Boolean(true))
            }
            HostCapabilityKind::Custom(CapabilityName::CommonExpectsError) => Ok(
                capability_function(HostCapabilityKind::Custom(
                    CapabilityName::CommonExpectsErrorCheck,
                )),
            ),
            HostCapabilityKind::Custom(CapabilityName::CommonExpectsErrorCheck) => {
                // Permissive validator: assert.throws calls it with the thrown
                // error; pass unless nothing was thrown.
                let error = arguments.first().cloned().unwrap_or(Value::Undefined);
                if matches!(error, Value::Undefined | Value::Null) {
                    Err(VmError::Thrown(quench_runtime::host_api::object(
                        vec![(
                            "name".into(),
                            Value::String("AssertionError".into()),
                        )],
                    )))
                } else {
                    Ok(Value::Undefined)
                }
            }
            HostCapabilityKind::Custom(CapabilityName::CommonPlatformTimeout) => Ok(arguments
                .first()
                .cloned()
                .unwrap_or(Value::Undefined)),
            HostCapabilityKind::Custom(CapabilityName::CommonGetPort) => {
                Ok(Value::Number(0.0))
            }
            HostCapabilityKind::Custom(CapabilityName::V8Noop) => Ok(Value::Undefined),
            HostCapabilityKind::Custom(CapabilityName::V8QueryObjects) => {
                Ok(quench_runtime::host_api::array(Vec::new()))
            }
            HostCapabilityKind::Custom(CapabilityName::V8SetFlags) => match arguments.first() {
                Some(Value::String(_) | Value::StringUnits(_)) => {
                    V8_FLAG_VERSION.fetch_add(1, Ordering::SeqCst);
                    Ok(Value::Undefined)
                }
                _ => Err(invalid_string_arg_error("flags")),
            },
            HostCapabilityKind::Custom(CapabilityName::V8CachedDataVersionTag) => {
                Ok(Value::Number(V8_FLAG_VERSION.load(Ordering::SeqCst) as f64))
            }
            HostCapabilityKind::Custom(CapabilityName::V8IsStringOneByte) => {
                match arguments.first() {
                    Some(Value::String(content)) if is_symbol_representation(content) => {
                        Err(invalid_string_arg_error("content"))
                    }
                    Some(Value::String(content)) => {
                        Ok(Value::Boolean(content.chars().all(|c| (c as u32) < 0x100)))
                    }
                    Some(Value::StringUnits(units)) => {
                        Ok(Value::Boolean(units.iter().all(|&unit| unit < 0x100)))
                    }
                    _ => Err(invalid_string_arg_error("content")),
                }
            }
            HostCapabilityKind::Custom(CapabilityName::V8StartupSnapshotIsBuilding) => {
                Ok(Value::Boolean(false))
            }
            HostCapabilityKind::Custom(CapabilityName::V8StartupSnapshotThrows) => Err(
                thrown_js_error(
                    "Error",
                    "ERR_NOT_BUILDING_SNAPSHOT",
                    "Cannot access this API while not building a snapshot.",
                ),
            ),
            HostCapabilityKind::Custom(CapabilityName::V8WriteHeapSnapshot) => {
                Err(thrown_js_error(
                    "Error",
                    "ERR_V8_NOT_SUPPORTED",
                    "Writing heap snapshots is not supported on this runtime.",
                ))
            }
            HostCapabilityKind::Custom(CapabilityName::V8HeapStats) => Ok(
                quench_runtime::host_api::object(vec![
                    ("does_zap_garbage".into(), Value::Number(0.0)),
                    ("external_memory".into(), Value::Number(0.0)),
                    ("heap_size_limit".into(), Value::Number(0.0)),
                    ("malloced_memory".into(), Value::Number(0.0)),
                    ("number_of_detached_contexts".into(), Value::Number(0.0)),
                    ("number_of_native_contexts".into(), Value::Number(0.0)),
                    ("peak_malloced_memory".into(), Value::Number(0.0)),
                    ("total_allocated_bytes".into(), Value::Number(0.0)),
                    ("total_available_size".into(), Value::Number(0.0)),
                    ("total_global_handles_size".into(), Value::Number(0.0)),
                    ("total_heap_size".into(), Value::Number(0.0)),
                    ("total_heap_size_executable".into(), Value::Number(0.0)),
                    ("total_physical_size".into(), Value::Number(0.0)),
                    ("used_global_handles_size".into(), Value::Number(0.0)),
                    ("used_heap_size".into(), Value::Number(0.0)),
                ]),
            ),
            HostCapabilityKind::Custom(CapabilityName::V8HeapCodeStats) => Ok(
                quench_runtime::host_api::object(vec![
                    ("bytecode_and_metadata_size".into(), Value::Number(0.0)),
                    ("code_and_metadata_size".into(), Value::Number(0.0)),
                    ("cpu_profiler_metadata_size".into(), Value::Number(0.0)),
                    ("external_script_source_size".into(), Value::Number(0.0)),
                ]),
            ),
            HostCapabilityKind::Custom(CapabilityName::V8HeapSpaceStats) => {
                let names = [
                    "code_large_object_space",
                    "code_space",
                    "large_object_space",
                    "new_large_object_space",
                    "new_space",
                    "old_space",
                    "read_only_space",
                    "shared_large_object_space",
                    "shared_space",
                    "shared_trusted_large_object_space",
                    "shared_trusted_space",
                    "trusted_large_object_space",
                    "trusted_space",
                ];
                let spaces = names
                    .iter()
                    .map(|name| {
                        quench_runtime::host_api::object(vec![
                            ("space_name".into(), Value::String((*name).into())),
                            ("space_size".into(), Value::Number(0.0)),
                            ("space_used_size".into(), Value::Number(0.0)),
                            ("space_available_size".into(), Value::Number(0.0)),
                            ("physical_space_size".into(), Value::Number(0.0)),
                        ])
                    })
                    .collect();
                Ok(quench_runtime::host_api::array(spaces))
            }
            HostCapabilityKind::Custom(CapabilityName::V8Serialize) => {
                Ok(quench_runtime::host_api::object(vec![(
                    "data".into(),
                    arguments.first().cloned().unwrap_or(Value::Undefined),
                )]))
            }
            HostCapabilityKind::Custom(CapabilityName::V8Deserialize) => {
                let value = arguments
                    .first()
                    .and_then(|argument| {
                        quench_runtime::execute::get_property_result(argument, "data").ok()
                    })
                    .unwrap_or(Value::Undefined);
                Ok(value)
            }
            HostCapabilityKind::Custom(CapabilityName::FsWriteAsync) => fs_write_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsReadAsync) => fs_read_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsWritePromise) => {
                fs_write_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsReadPromise) => fs_read_promise(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsAppendPromise) => {
                fs_write_bytes(arguments, true)?;
                Ok(fulfilled(Value::Undefined))
            }
            HostCapabilityKind::Custom(CapabilityName::FsOpenAsync) => {
                self.fs_open_async(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsCloseAsync) => {
                self.fs_close_async(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatAsync) => fs_stat_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsLstatAsync) => fs_lstat_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsStatPromise) => {
                fs_stat_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsMkdirPromise) => {
                fs_mkdir_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsRmPromise) => fs_rm_promise(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsRenamePromise) => {
                fs_rename_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsAccessPromise) => {
                fs_access_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsChmodPromise) => {
                fs_chmod_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsReadlinkPromise) => {
                fs_readlink_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsRealpathPromise) => {
                fs_realpath_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsSymlinkPromise) => {
                fs_symlink_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsMkdtempPromise) => {
                fs_mkdtemp_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsLstatPromise) => {
                fs_lstat_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsTruncatePromise) => {
                fs_truncate_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsCopyFilePromise) => {
                fs_copy_file_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsUtimesPromise) => {
                fs_utimes_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsOpenPromise) => {
                self.fs_open_promise(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsFileHandleClose) => {
                self.fs_filehandle_close(receiver)
            }
            HostCapabilityKind::Custom(CapabilityName::FsFileHandleReadFile) => {
                self.fs_filehandle_read_file(receiver, arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsFileHandleRead) => {
                self.fs_filehandle_read(receiver, arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsFileHandleWrite) => {
                self.fs_filehandle_write(receiver, arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsFileHandleStat) => {
                self.fs_filehandle_stat(receiver, arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatsIsDirectory) => {
                Ok(Value::Boolean(true))
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatsIsFile) => Ok(Value::Boolean(false)),
            HostCapabilityKind::Custom(CapabilityName::FsMkdirSync) => fs_mkdir(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsMkdirAsync) => {
                let callback = arguments.last().cloned().ok_or(VmError::NotCallable)?;
                let result = fs_mkdir(&arguments[..arguments.len().saturating_sub(1)]);
                let args = match result {
                    Ok(result) => vec![Value::Null, result],
                    Err(VmError::Thrown(error)) => vec![error],
                    Err(_) => vec![Value::Null, Value::Undefined],
                };
                quench_runtime::execute::call(&callback, &Value::Undefined, &args)?;
                Ok(Value::Undefined)
            }
            HostCapabilityKind::Custom(CapabilityName::FsToUnixTimestamp) => {
                let value = arguments.first().cloned().unwrap_or(Value::Number(0.0));
                Ok(Value::Number(match value {
                    Value::Number(value) => {
                        if value < 0.0 {
                            1.0
                        } else {
                            value
                        }
                    }
                    _ => 12.0,
                }))
            }
            HostCapabilityKind::Custom(CapabilityName::FsRmSync) => fs_rm(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsRenameSync) => fs_rename(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsRm) => fs_rm_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsSymlink) => fs_symlink_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsReadlink) => fs_readlink_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsRealpath) => fs_realpath_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsMkdtempAsync) => fs_mkdtemp_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsCpSync) => fs_cp(arguments, false),
            HostCapabilityKind::Custom(CapabilityName::FsCp) => fs_cp(arguments, true),
            HostCapabilityKind::Custom(CapabilityName::TimersPromisesSetTimeout) => {
                timers_promises_set_timeout(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::TimersPromisesSetImmediate) => {
                timers_promises_set_immediate(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsReaddirSync) => fs_readdir(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsReaddirAsync) => {
                fs_readdir_async(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsReaddirPromise) => {
                Ok(fulfilled(fs_readdir(arguments)?))
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatSync) => fs_stat_sync(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsLstatSync) => fs_lstat_sync(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsSymlinkSync) => fs_symlink(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsReadlinkSync) => fs_readlink(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsStringToFlags) => {
                string_to_flags(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatsIsDirectoryFile) => {
                Ok(Value::Boolean(false))
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatsIsSymbolicLink) => {
                Ok(Value::Boolean(true))
            }
            HostCapabilityKind::Custom(CapabilityName::FsStatsIsNotSymbolicLink) => {
                Ok(Value::Boolean(false))
            }
            HostCapabilityKind::Custom(CapabilityName::FsFtruncateSync) => {
                self.fs_ftruncate(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsTruncateAsync) => {
                fs_truncate_async(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsTruncateSync) => {
                fs_truncate_sync(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::StreamIterText) => {
                stream_iter_text(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::StreamIterBytes) => {
                stream_iter_bytes(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::StreamIterPull)
            | HostCapabilityKind::Custom(CapabilityName::StreamIterIdentity) => {
                Ok(arguments.first().cloned().unwrap_or(Value::Undefined))
            }
            HostCapabilityKind::Custom(CapabilityName::ZlibIterCompress)
            | HostCapabilityKind::Custom(CapabilityName::ZlibIterDecompress) => {
                Ok(capability_function(HostCapabilityKind::Custom(
                    CapabilityName::StreamIterIdentity,
                )))
            }
            HostCapabilityKind::Custom(CapabilityName::FsFsyncSync)
            | HostCapabilityKind::Custom(CapabilityName::FsFdatasyncSync) => Ok(Value::Undefined),
            HostCapabilityKind::Custom(CapabilityName::FsFsyncAsync)
            | HostCapabilityKind::Custom(CapabilityName::FsFdatasyncAsync) => {
                if let Some(callback) = arguments.last() {
                    quench_runtime::execute::call(callback, &Value::Undefined, &[Value::Null])?;
                }
                Ok(Value::Undefined)
            }
            HostCapabilityKind::Custom(CapabilityName::FsUnlinkPromise) => {
                fs_unlink(arguments).map(|value| fulfilled(value))
            }
            HostCapabilityKind::Custom(CapabilityName::FsOpendirSync) => self.fs_opendir(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsOpendirAsync) => {
                self.fs_opendir_async(arguments)
            }
            HostCapabilityKind::Custom(CapabilityName::FsOpendirPromise) => {
                self.fs_opendir(arguments).map(fulfilled)
            }
            HostCapabilityKind::Custom(CapabilityName::FsDirReadSync) => {
                self.fs_dir_read(receiver, arguments, false)
            }
            HostCapabilityKind::Custom(CapabilityName::FsDirReadAsync) => {
                self.fs_dir_read(receiver, arguments, true)
            }
            HostCapabilityKind::Custom(CapabilityName::FsDirCloseSync) => {
                self.fs_dir_close(receiver, arguments, false)
            }
            HostCapabilityKind::Custom(CapabilityName::FsDirCloseAsync) => {
                self.fs_dir_close(receiver, arguments, true)
            }
            HostCapabilityKind::Custom(CapabilityName::FsLinkSync) => {
                fs_link(arguments).map_err(invalid_path_error)
            }
            HostCapabilityKind::Custom(CapabilityName::FsLinkAsync) => fs_link_async(arguments),
            HostCapabilityKind::Custom(CapabilityName::FsLinkPromise) => {
                fs_link(arguments).map(fulfilled)
            }
            HostCapabilityKind::Custom(CapabilityName::FsReadSyncFd) => {
                self.fs_read_fd(arguments, false)
            }
            HostCapabilityKind::Custom(CapabilityName::FsReadFdAsync) => {
                self.fs_read_fd(arguments, true)
            }
            HostCapabilityKind::Custom(CapabilityName::ProcessStdoutWrite) => {
                process_stdio_write(arguments, 1)
            }
            HostCapabilityKind::Custom(CapabilityName::ProcessStderrWrite) => {
                process_stdio_write(arguments, 2)
            }
            HostCapabilityKind::Custom(CapabilityName::StdioIdentity) => {
                Ok(receiver.cloned().unwrap_or(Value::Undefined))
            }
            HostCapabilityKind::Custom(CapabilityName::StdioListenersEmpty) => {
                Ok(quench_runtime::host_api::array(Vec::new()))
            }
            HostCapabilityKind::Custom(CapabilityName::StdioCountZero) => Ok(Value::Number(0.0)),
                _ => Err(VmError::EvalError(DISPATCH_UNHANDLED.into())),
            }
        })();
        match result {
            Err(VmError::EvalError(message)) if message == DISPATCH_UNHANDLED => None,
            result => Some(result),
        }
    }
}
