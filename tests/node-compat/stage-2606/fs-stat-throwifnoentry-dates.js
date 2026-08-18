"use strict";

const assert = require("assert");
const fs = require("fs");

// statSync/stat with throwIfNoEntry:false return undefined for a missing file.
assert.strictEqual(
  fs.statSync("./definitely_not_present_q", { throwIfNoEntry: false }),
  undefined
);
fs.stat(
  "./definitely_not_present_q",
  { throwIfNoEntry: false },
  (err, stats) => {
    assert.strictEqual(err, null);
    assert.strictEqual(stats, undefined);
  }
);

// Stat time fields are Date instances from real metadata.
fs.stat(__filename, (err, stats) => {
  assert.strictEqual(err, null);
  assert.ok(stats.mtime instanceof Date);
  assert.ok(stats.atime instanceof Date);
  assert.ok(stats.ctime instanceof Date);
  assert.ok(stats.birthtime instanceof Date);
});

console.log("fs stat throwIfNoEntry/Date passed");
