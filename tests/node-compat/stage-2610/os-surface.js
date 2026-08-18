"use strict";

const assert = require("assert");
const os = require("os");

process.env.TMPDIR = "/tmpdir";
process.env.TMP = "/tmp";
process.env.TEMP = "/temp";
assert.strictEqual(os.tmpdir(), "/tmpdir");
process.env.TMPDIR = "";
assert.strictEqual(os.tmpdir(), "/tmp");
process.env.TMP = "";
assert.strictEqual(os.tmpdir(), "/temp");
process.env.TEMP = "";
assert.strictEqual(os.tmpdir(), "/tmp");
process.env.TMPDIR = "/";
assert.strictEqual(os.tmpdir(), "/");
assert.strictEqual(`${os.tmpdir}`, os.tmpdir());

assert.ok(["arm64", "x64", "ia32"].includes(os.arch()) || os.arch().length > 0);
assert.match(os.endianness(), /[BL]E/);
assert.ok(os.hostname().length > 0);
assert.ok(os.release().length > 0);
assert.ok(os.version().length > 0);
assert.ok(os.cpus().length > 0);
assert.strictEqual(typeof os.cpus()[0].model, "string");
assert.strictEqual(typeof os.cpus()[0].times.user, "number");
assert.ok(os.totalmem() > 0);
assert.ok(os.freemem() > 0);
assert.strictEqual(os.devNull, "/dev/null");
assert.strictEqual(os.EOL, "\n");

const { PRIORITY_BELOW_NORMAL, PRIORITY_LOW } = os.constants.priority;
const lower =
  os.getPriority() < PRIORITY_BELOW_NORMAL
    ? PRIORITY_BELOW_NORMAL
    : PRIORITY_LOW;
os.setPriority(lower);
assert.strictEqual(os.getPriority(), lower);

console.log("os surface passed");
