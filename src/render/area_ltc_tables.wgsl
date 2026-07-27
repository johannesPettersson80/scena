// LTC lookup tables for analytic area-light specular.
//
// Generated from selfshadow/ltc_code fit/results/ltc.js (master blob sha 90c2ae903e5e460c03f28bc14d0391dba9578e71).
// Source: https://raw.githubusercontent.com/selfshadow/ltc_code/master/fit/results/ltc.js
// Compact 16x16 table: each texel is bilinearly resampled from the published 64x64 reference table.
//
// The table data lives in Rust (`src/render/area_ltc_tables.rs`, shared with the
// CPU renderer) and is uploaded into a uniform buffer rather than baked into this
// module as `const` arrays.
//
// A runtime index into a module-scope `const` array cannot be expressed by
// hardware without an indexed constant-register file, so the driver expands each
// read into a select chain over every element. Two 256-entry tables read eight
// times per fragment made V3D's register allocator fail at all thirteen of its
// fallback strategies and produced a 22,518-instruction fragment shader against
// a 74-instruction median for every other shader in the same build. A uniform
// block is a memory load instead, costs no texture unit or sampler, and stays
// inside WebGL2's 16 KiB `max_uniform_buffer_binding_size` at 8 KiB.

const LTC_LUT_SIZE_F: f32 = 16.0;
const LTC_LUT_LAST_F: f32 = 15.0;
const LTC_LUT_LAST_U: u32 = 15u;
const LTC_LUT_STRIDE_U: u32 = 16u;

// Row-major 16x16, matching the Rust `[[[f32; 4]; 16]; 16]` flattened as
// `y * 16 + x`. `array<vec4<f32>, N>` has a std140 stride of 16 bytes, so the
// Rust and WGSL layouts are byte-identical with no padding.
struct LtcTables {
    table_1: array<vec4<f32>, 256>,
    table_2: array<vec4<f32>, 256>,
};

@group(0) @binding(10) var<uniform> ltc_tables: LtcTables;
