#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(all(not(target_arch = "wasm32"), feature = "glutin"))]
pub mod glutin;

#[cfg(all(not(target_arch = "wasm32"), feature = "surfman"))]
pub mod surfman;

#[cfg(all(not(target_arch = "wasm32"), feature = "wgl"))]
pub mod wgl;

#[cfg(not(any(
    target_arch = "wasm32",
    feature = "glutin",
    feature = "surfman",
    feature = "wgl"
)))]
pub mod dummy;
