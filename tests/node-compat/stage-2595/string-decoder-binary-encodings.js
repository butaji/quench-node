"use strict";

const assert = require("assert");
const { StringDecoder } = require("string_decoder");

// base64 / base64url / hex are accepted encodings and stream with 3-byte
// (base64) group alignment across writes.
function writeAll(enc, bufs) {
  const s = new StringDecoder(enc);
  let out = "";
  for (const buf of bufs) out += s.write(buf);
  out += s.end();
  return out;
}

assert.strictEqual(writeAll("base64", [Buffer.of(0x61)]), "YQ==");
assert.strictEqual(
  writeAll("base64", [Buffer.of(0x61), Buffer.of(0x61)]),
  "YWE="
);
assert.strictEqual(writeAll("base64", [Buffer.from("asdf")]), "YXNkZg==");
assert.strictEqual(writeAll("base64url", [Buffer.of(0x61)]), "YQ");
assert.strictEqual(writeAll("base64url", [Buffer.from("aa")]), "YWE");
assert.strictEqual(writeAll("hex", [Buffer.of(0x61)]), "61");
assert.strictEqual(writeAll("hex", [Buffer.from("asdf")]), "61736466");

// Decoder.end() flushes pending incomplete groups.
{
  const s = new StringDecoder("base64");
  assert.strictEqual(s.write(Buffer.of(0x61)), "");
  assert.strictEqual(s.end(), "YQ==");
}
{
  const s = new StringDecoder("hex");
  assert.strictEqual(s.write(Buffer.of(0x61)), "61");
  assert.strictEqual(s.end(), "");
}

// utf8 keeps an incomplete trailing sequence pending and flushes U+FFFD.
{
  const s = new StringDecoder();
  assert.strictEqual(s.write(Buffer.from("E1", "hex")), "");
  assert.strictEqual(s.end(), "\uFFFD");
}
{
  // UTF-16LE surrogate pair delivered across writes.
  const s = new StringDecoder("utf16le");
  assert.strictEqual(s.write(Buffer.from("3DD8", "hex")), "");
  assert.strictEqual(s.write(Buffer.from("4D", "hex")), "");
  assert.strictEqual(s.write(Buffer.from("DC", "hex")), "\uD83D\uDC4D");
  assert.strictEqual(s.end(), "");
}

// Unknown encodings are rejected.
assert.throws(() => new StringDecoder("nope"), {
  code: "ERR_UNKNOWN_ENCODING"
});

console.log("string_decoder base64/hex/end passed");
