export { Schema } from "./schema.js";
export { BitspecError, type BitspecErrorCode } from "./error.js";
export { init } from "./wasm.js";
export { floatBits32, floatBits64 } from "./helpers.js";
export type {
  Value,
  SchemaDef,
  FieldDef,
  FragmentDef,
  FieldKindDef,
  AssembleDef,
  BitOrderDef,
  TransformDef,
  BaseDef,
  EncodingDef,
  WriteConfigDef,
} from "./types.js";
