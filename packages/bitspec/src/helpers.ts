import type { Value } from "./types.js";

/** Returns a `Value::U64` containing the raw 32-bit representation of `x`. */
export function floatBits32(x: number): Value {
  const buf = new ArrayBuffer(4);
  new Float32Array(buf)[0] = x;
  const u = new Uint32Array(buf)[0];
  return { kind: "u64", value: BigInt(u) };
}

/** Returns a `Value::U64` containing the raw 64-bit representation of `x`. */
export function floatBits64(x: number): Value {
  const buf = new ArrayBuffer(8);
  new Float64Array(buf)[0] = x;
  const u32s = new Uint32Array(buf);
  const lo = BigInt(u32s[0]);
  const hi = BigInt(u32s[1]);
  return { kind: "u64", value: (hi << 32n) | lo };
}
