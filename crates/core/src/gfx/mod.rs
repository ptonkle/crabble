//! What the 3D renderer reads, in fixed point (BUILDER-SPEC D2: no floats
//! in the core). Ported from Honey Hole's `gfx` (SECOND-ORDER R1.60).
//!
//! - `fx`: Q16.16 numbers and integer angles (`ONE` is one metre, one
//!   second, 1.0);
//! - `geom`: vectors, affine transforms and 4×4 matrices, +y up;
//! - `render`: the vertex layout, the camera, the light and the water's
//!   look as uniforms, and a frame for the page to draw.

pub mod fx;
pub mod geom;
pub mod render;
