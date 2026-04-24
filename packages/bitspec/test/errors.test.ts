import { beforeAll, describe, expect, it } from "vitest";
import { BitspecError, init, Schema } from "../src/index.js";

beforeAll(async () => { await init(); });

describe("BitspecError", () => {
  it("throws PACKET_TOO_SHORT when input is too small", () => {
    const schema = Schema.compile({
      fields: [
        { name: "x", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 0, len_bits: 16 }] },
      ],
    });
    try {
      schema.parse(new Uint8Array([0x01]));
      expect.fail("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(BitspecError);
      expect((e as BitspecError).code).toBe("PACKET_TOO_SHORT");
    }
  });

  it("throws MISSING_FIELD when serialize is missing a value", () => {
    const schema = Schema.compile({
      fields: [
        { name: "x", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
          fragments: [{ offset_bits: 0, len_bits: 8 }] },
      ],
    });
    try {
      schema.serialize({});
      expect.fail("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(BitspecError);
      expect((e as BitspecError).code).toBe("MISSING_FIELD");
    }
  });

  it("throws SCHEMA_JSON_PARSE_ERROR on invalid JSON", () => {
    try {
      Schema.compile("{not valid json");
      expect.fail("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(BitspecError);
      expect((e as BitspecError).code).toBe("SCHEMA_JSON_PARSE_ERROR");
    }
  });
});
