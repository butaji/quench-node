"use strict";

const assert = require("assert");
const v8 = require("v8");

// getHeapStatistics() exposes the canonical Node/V8 key set, all numbers.
const heapKeys = [
  "does_zap_garbage",
  "external_memory",
  "heap_size_limit",
  "malloced_memory",
  "number_of_detached_contexts",
  "number_of_native_contexts",
  "peak_malloced_memory",
  "total_allocated_bytes",
  "total_available_size",
  "total_global_handles_size",
  "total_heap_size",
  "total_heap_size_executable",
  "total_physical_size",
  "used_global_handles_size",
  "used_heap_size"
];
const stats = v8.getHeapStatistics();
assert.deepStrictEqual(Object.keys(stats).sort(), heapKeys);
for (const key of heapKeys) {
  assert.strictEqual(typeof stats[key], "number");
}

// getHeapCodeStatistics() exposes the four code-stat keys.
const codeStats = v8.getHeapCodeStatistics();
assert.deepStrictEqual(Object.keys(codeStats).sort(), [
  "bytecode_and_metadata_size",
  "code_and_metadata_size",
  "cpu_profiler_metadata_size",
  "external_script_source_size"
]);
for (const key of Object.keys(codeStats)) {
  assert.strictEqual(typeof codeStats[key], "number");
}

// getHeapSpaceStatistics() lists the canonical space names with numeric fields.
const heapSpaces = v8.getHeapSpaceStatistics();
assert.deepStrictEqual(heapSpaces.map((s) => s.space_name).sort(), [
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
  "trusted_space"
]);
for (const space of heapSpaces) {
  assert.strictEqual(typeof space.space_name, "string");
  assert.strictEqual(typeof space.space_size, "number");
  assert.strictEqual(typeof space.space_used_size, "number");
  assert.strictEqual(typeof space.space_available_size, "number");
  assert.strictEqual(typeof space.physical_space_size, "number");
}

// cachedDataVersionTag() is a stable number that changes across setFlags.
const versionTag1 = v8.cachedDataVersionTag();
assert.strictEqual(typeof versionTag1, "number");
assert.strictEqual(v8.cachedDataVersionTag(), versionTag1);
v8.setFlagsFromString("--allow_natives_syntax");
assert.notStrictEqual(v8.cachedDataVersionTag(), versionTag1);

// setFlagsFromString() rejects non-string arguments.
for (const value of [1, undefined, null, {}]) {
  assert.throws(() => v8.setFlagsFromString(value), {
    code: "ERR_INVALID_ARG_TYPE",
    name: "TypeError",
    message: 'The "flags" argument must be of type string.'
  });
}

// isStringOneByteRepresentation() reports one-byte-representable strings and
// rejects non-strings (including symbols).
assert.strictEqual(v8.isStringOneByteRepresentation("hello world!"), true);
assert.strictEqual(v8.isStringOneByteRepresentation("你好😀😃"), false);
for (const value of [undefined, null, false, 5n, 5, Symbol(), () => {}, {}]) {
  assert.throws(
    () => v8.isStringOneByteRepresentation(value),
    /The "content" argument must be of type string/
  );
}

// startupSnapshot APIs are guarded when not building a snapshot.
assert.strictEqual(v8.startupSnapshot.isBuildingSnapshot(), false);
for (const fn of [
  "addSerializeCallback",
  "addDeserializeCallback",
  "setDeserializeMainFunction"
]) {
  assert.throws(() => v8.startupSnapshot[fn](() => {}), {
    code: "ERR_NOT_BUILDING_SNAPSHOT"
  });
}

// serialize/deserialize round-trip preserves values.
assert.deepStrictEqual(
  v8.deserialize(v8.serialize({ a: new Int32Array(1024) })),
  { a: new Int32Array(1024) }
);
assert.strictEqual(v8.deserialize(v8.serialize(Buffer.alloc(0))).length, 0);

console.log("v8 module surface passed");
