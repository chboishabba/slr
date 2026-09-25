// Source-family-neutral entry point for the SCALE-1 compiler.
//
// The implementation is intentionally shared byte-for-byte with the historical
// long-document example while source-family adapters converge on the generic
// compile-source surface.  Keeping this as an include avoids a second CLI
// implementation and preserves the old example as a compatibility entry point.
include!("scale1_long_document.rs");
