export type BitspecErrorCode =
  | "INVALID_ARRAY_STRIDE" | "INVALID_ARRAY_COUNT" | "INVALID_FIELD_SIZE"
  | "INVALID_FRAGMENT"     | "INVALID_FIELD_KIND"  | "EMPTY_ARRAY_ELEMENT"
  | "INVALID_FIELD_NAME"
  | "READ_OUT_OF_BOUNDS"   | "TOO_MANY_BITS_READ"  | "PACKET_TOO_SHORT"
  | "WRITE_OUT_OF_BOUNDS"  | "INVALID_VALUE"       | "MISSING_FIELD"
  | "UNSUPPORTED_VALUE"
  | "INVALID_BASE"         | "INVALID_TYPE"        | "INVALID_ENUM_VALUE"
  | "INVALID_ENCODING"     | "INVALID_BYTE_VALUE"  | "INVALID_ASCII_BYTE_VALUE"
  | "INVALID_SCALE_OFFSET"
  | "SCHEMA_JSON_PARSE_ERROR" | "INPUT_CONVERSION_ERROR";

const KNOWN_CODES = new Set<BitspecErrorCode>([
  "INVALID_ARRAY_STRIDE", "INVALID_ARRAY_COUNT", "INVALID_FIELD_SIZE",
  "INVALID_FRAGMENT", "INVALID_FIELD_KIND", "EMPTY_ARRAY_ELEMENT",
  "INVALID_FIELD_NAME",
  "READ_OUT_OF_BOUNDS", "TOO_MANY_BITS_READ", "PACKET_TOO_SHORT",
  "WRITE_OUT_OF_BOUNDS", "INVALID_VALUE", "MISSING_FIELD",
  "UNSUPPORTED_VALUE",
  "INVALID_BASE", "INVALID_TYPE", "INVALID_ENUM_VALUE",
  "INVALID_ENCODING", "INVALID_BYTE_VALUE", "INVALID_ASCII_BYTE_VALUE",
  "INVALID_SCALE_OFFSET",
  "SCHEMA_JSON_PARSE_ERROR", "INPUT_CONVERSION_ERROR",
]);

export class BitspecError extends Error {
  constructor(public readonly code: BitspecErrorCode, message: string) {
    super(`[${code}] ${message}`);
    this.name = "BitspecError";
  }
}

export function translateError(raw: unknown): BitspecError {
  if (
    raw !== null &&
    typeof raw === "object" &&
    "code" in raw &&
    "message" in raw &&
    typeof (raw as { code: unknown }).code === "string" &&
    typeof (raw as { message: unknown }).message === "string"
  ) {
    const code = (raw as { code: string }).code;
    const message = (raw as { message: string }).message;
    if (KNOWN_CODES.has(code as BitspecErrorCode)) {
      return new BitspecError(code as BitspecErrorCode, message);
    }
    return new BitspecError("INVALID_VALUE", `unknown code '${code}': ${message}`);
  }
  return new BitspecError("INVALID_VALUE", `unexpected boundary error: ${String(raw)}`);
}
