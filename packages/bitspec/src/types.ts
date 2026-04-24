/**
 * Unified value type used by parse input/output and serialize input.
 * Mirrors the Rust `Value` enum with externally-tagged serde shape.
 */
export type Value =
  | { kind: "u64"; value: bigint }
  | { kind: "i64"; value: bigint }
  | { kind: "f32"; value: number }
  | { kind: "f64"; value: number }
  | { kind: "bytes"; value: Uint8Array }
  | { kind: "string"; value: string }
  | { kind: "array"; value: Value[] };

/** Bit order used when reading/writing a fragment. */
export type BitOrderDef = "MsbFirst" | "LsbFirst";

/** How fragments are concatenated. */
export type AssembleDef = "ConcatMsb" | "ConcatLsb";

/** A contiguous bit range within the payload. */
export interface FragmentDef {
  offset_bits: number;
  len_bits: number;
  bit_order?: BitOrderDef;
}

/** Scalar or fixed-size array field kind. */
export type FieldKindDef =
  | { type: "Scalar" }
  | { type: "Array"; count: number; stride_bits: number; offset_bits: number };

/** Transform base type. */
export type BaseDef = "Int" | "Float32" | "Float64" | "Bytes";

/** Text encoding applied to a Bytes base. */
export type EncodingDef = "Utf8" | "Ascii";

/** Transform configuration attached to a field. */
export interface TransformDef {
  base: BaseDef;
  scale?: number;
  offset?: number;
  encoding?: EncodingDef;
  zero_terminated?: boolean;
  trim?: boolean;
  enum_map?: Record<number, string>;
}

/** A single field in the schema. */
export interface FieldDef {
  name: string;
  kind: FieldKindDef;
  signed: boolean;
  assemble: AssembleDef;
  fragments: FragmentDef[];
  transform?: TransformDef;
}

/** Write configuration for serialize. */
export interface WriteConfigDef {
  bit_order?: BitOrderDef;
}

/** Top-level schema definition. */
export interface SchemaDef {
  fields: FieldDef[];
  write_config?: WriteConfigDef;
}
