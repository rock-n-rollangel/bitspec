import { beforeAll, describe, expect, it } from "vitest";
import { init, Schema, type SchemaDef } from "../src/index.js";

beforeAll(async () => { await init(); });

describe("roundtrip", () => {
  it("parses what it serialized for a two-field schema", () => {
    const def: SchemaDef = {
      fields: [
        { name: "a", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 0, len_bits: 4 }] },
        { name: "b", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 4, len_bits: 4 }] },
      ],
    };
    const schema = Schema.compile(def);
    const bytes = schema.serialize({
      a: { kind: "u64", value: 0b1010n },
      b: { kind: "u64", value: 0b0101n },
    });
    expect(bytes).toEqual(new Uint8Array([0b1010_0101]));
    const parsed = schema.parse(bytes);
    expect(parsed.a).toEqual({ kind: "u64", value: 0b1010n });
    expect(parsed.b).toEqual({ kind: "u64", value: 0b0101n });
  });
});
