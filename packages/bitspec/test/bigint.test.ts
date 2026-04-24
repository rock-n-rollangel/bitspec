import { beforeAll, describe, expect, it } from "vitest";
import { init, Schema } from "../src/index.js";

beforeAll(async () => { await init(); });

describe("bigint", () => {
  it("preserves u64 values above 2^53 through roundtrip", () => {
    const schema = Schema.compile({
      fields: [
        { name: "n", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 0, len_bits: 64 }] },
      ],
    });
    const large = (1n << 60n) + 12345n;
    const bytes = schema.serialize({ n: { kind: "u64", value: large } });
    const parsed = schema.parse(bytes);
    expect(parsed.n).toEqual({ kind: "u64", value: large });
  });
});
