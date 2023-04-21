use crate::ffi::*;
use crate::retro::*;
use c_utf8::CUtf8;
use core::ffi::*;

/// Marker trait for types that are valid arguments to the environment callback.
///
/// Any type implementing this trait must be FFI-safe. Structs should be `#[repr(C)]` or a
/// `#[repr(transparent)]` newtype. Numeric enums should have the appropriate primitive
/// representation, which is typically either `#[repr(core::ffi::c_uint)]` for
/// `const unsigned` arguments or `#[repr(core::ffi::c_int)]` for `const enum` arguments.
///
/// Care must still be taken when calling any of the generic unsafe `[RetroEnvironment]` methods to
/// ensure the type used is appropriate for the environment command, as specified in `libretro.h`.
pub trait CommandData {}
impl CommandData for () {}
impl CommandData for bool {}
impl CommandData for c_int {}
impl CommandData for c_uint {}
impl CommandData for Option<&c_char> {}
impl CommandData for Option<&c_void> {}
impl CommandData for retro_hw_render_callback {}
impl CommandData for retro_game_geometry {}
impl CommandData for GameGeometry {}
impl CommandData for retro_log_callback {}
impl CommandData for retro_message {}
impl CommandData for Message {}
impl CommandData for retro_system_av_info {}
impl CommandData for SystemAVInfo {}
impl CommandData for retro_variable {}

/// Unsafe type conversions.
pub trait UnsafeFrom<T> {
  unsafe fn unsafe_from(x: T) -> Self;
}

pub trait UnsafeInto<T> {
  unsafe fn unsafe_into(self) -> T;
}

impl<T, U, E> UnsafeFrom<Result<U, E>> for Result<T, E>
where
  T: UnsafeFrom<U>,
{
  unsafe fn unsafe_from(x: Result<U, E>) -> Self {
    x.map(|ok| T::unsafe_from(ok))
  }
}

impl<T, U> UnsafeInto<U> for T
where
  U: UnsafeFrom<T>,
{
  unsafe fn unsafe_into(self) -> U {
    U::unsafe_from(self)
  }
}

impl<'a> UnsafeFrom<Option<&'a c_char>> for Option<&'a CStr> {
  unsafe fn unsafe_from(str: Option<&'a c_char>) -> Self {
    str.map(|ptr| CStr::from_ptr(ptr))
  }
}

impl<'a> UnsafeFrom<Option<&'a c_char>> for Option<&'a CUtf8> {
  unsafe fn unsafe_from(str: Option<&'a c_char>) -> Self {
    str.map(|ptr| CUtf8::from_c_str_unchecked(CStr::from_ptr(ptr)))
  }
}

impl UnsafeFrom<retro_log_callback> for PlatformLogger {
  unsafe fn unsafe_from(cb: retro_log_callback) -> Self {
    PlatformLogger::new(cb.log.unwrap())
  }
}

impl<'a> UnsafeFrom<retro_variable> for RetroVariable<'a> {
  unsafe fn unsafe_from(var: retro_variable) -> Self {
    Self(var.value.as_ref().map(|ptr| CStr::from_ptr(ptr)))
  }
}
