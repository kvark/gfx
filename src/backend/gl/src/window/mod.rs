#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(feature = "glutin")]
pub mod glutin;

#[cfg(feature = "surfman")]
pub mod surfman;

#[cfg(feature = "wgl")]
pub mod wgl;

#[cfg(not(any(
    target_arch = "wasm32",
    feature = "glutin",
    feature = "surfman",
    feature = "wgl"
)))]
pub mod dummy;
