"use strict";

const assert = require("assert");
const { StringDecoder } = require("string_decoder");

// All possible write splits of a UTF-8 input must produce the identical
// decoded string (Node's streaming contract), including invalid sequences.
function allSplits(encoding, bytes, expected) {
  function parts(len, prefix, out) {
    if (len === 0) return out.push(prefix);
    for (let k = 1; k <= len; k++) parts(len - k, prefix.concat([k]), out);
  }
  const splits = [];
  parts(bytes.length, [], splits);
  for (const split of splits) {
    const decoder = new StringDecoder(encoding);
    let out = "";
    let pos = 0;
    for (const k of split) {
      out += decoder.write(Buffer.from(bytes.slice(pos, pos + k)));
      pos += k;
    }
    out += decoder.end();
    assert.strictEqual(
      out,
      expected,
      `${encoding} split=${JSON.stringify(split)}`
    );
  }
}

// Invalid multi-byte sequences stay stable across every split.
allSplits("utf-8", Buffer.from("C9B5A941", "hex"), "\u0275\ufffdA");
allSplits("utf-8", Buffer.from("CCCCB8", "hex"), "\ufffd\u0338");
allSplits("utf-8", Buffer.from("F0B841", "hex"), "\ufffdA");
allSplits("utf-8", Buffer.from("E2B8CCB8", "hex"), "\ufffd\u0338");
allSplits("utf-8", Buffer.from("E2FBCC01", "hex"), "\ufffd\ufffd\ufffd\u0001");
allSplits("utf-8", Buffer.from("F0FB00", "hex"), "\ufffd\ufffd\0");
allSplits(
  "utf-8",
  Buffer.from("EDA0B5EDB08D", "hex"),
  "\ufffd\ufffd\ufffd\ufffd\ufffd\ufffd"
);
allSplits("utf-8", Buffer.from("CCB8CDB9", "hex"), "\u0338\u0379");

// A surrogate pair may be delivered a precise byte at a time.
{
  const decoder = new StringDecoder("utf16le");
  assert.strictEqual(decoder.write(Buffer.from("3DD8", "hex")), "");
  assert.strictEqual(decoder.write(Buffer.from("4D", "hex")), "");
  assert.strictEqual(decoder.write(Buffer.from("DC", "hex")), "\uD83D\uDC4D");
  assert.strictEqual(decoder.end(), "");
}

console.log("string_decoder utf-8 streaming passed");
