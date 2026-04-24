import { beforeAll, describe, expect, it } from "vitest";
import { init, Schema } from "../src/index.js";

beforeAll(async () => { await init(); });

describe("transforms", () => {
  it("applies scale+offset and produces F64", () => {
    const schema = Schema.compile({
      fields: [
        { name: "t", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 0, len_bits: 8 }],
          transform: { base: "Int", scale: 0.5, offset: 10 } },
      ],
    });
    const parsed = schema.parse(new Uint8Array([20]));
    expect(parsed.t).toEqual({ kind: "f64", value: 20 });
  });

  it("applies enum_map and produces String", () => {
    const schema = Schema.compile({
      fields: [
        { name: "s", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 0, len_bits: 8 }],
          transform: { base: "Int", enum_map: { 1: "one", 2: "two" } } },
      ],
    });
    const parsed = schema.parse(new Uint8Array([2]));
    expect(parsed.s).toEqual({ kind: "string", value: "two" });
  });
});
