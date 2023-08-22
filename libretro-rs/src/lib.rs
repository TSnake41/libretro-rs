pub use libc;

pub mod core_macro;
pub mod sys;

use libc::{c_char, c_void};
use std::ffi::CStr;
use sys::*;

#[allow(unused_variables)]
pub trait RetroCore {
  const SUPPORT_NO_GAME: bool = false;

  /// Called when a new instance of the core is needed. The `env` parameter can be used to set-up and/or query values
  /// required for the core to function properly.
  fn init(env: &RetroEnvironment) -> Self;

  /// Called to get information about the core. This information can then be displayed in a frontend, or used to
  /// construct core-specific paths.
  fn get_system_info() -> RetroSystemInfo;

  /// Called to associate a particular device with a particular port. A core is allowed to ignore this request.
  fn set_controller_port_device(&mut self, env: &RetroEnvironment, port: u32, device: RetroDevice) {}

  /// Called when a player resets their game.
  fn reset(&mut self, env: &RetroEnvironment);

  /// Called continuously once the core is initialized and a game is loaded. The core is expected to advance emulation
  /// by a single frame before returning.
  fn run(&mut self, env: &RetroEnvironment, runtime: &RetroRuntime);

  /// Called to determine the size of the save state buffer. This is only ever called once per run, and the core must
  /// not exceed the size returned here for subsequent saves.
  fn serialize_size(&self, env: &RetroEnvironment) -> usize {
    0
  }

  /// Allows a core to save its internal state into the specified buffer. The buffer is guaranteed to be at least `size`
  /// bytes, where `size` is the value returned from `serialize_size`.
  fn serialize(&self, env: &RetroEnvironment, data: *mut (), size: usize) -> bool {
    false
  }

  /// Allows a core to load its internal state from the specified buffer. The buffer is guaranteed to be at least `size`
  /// bytes, where `size` is the value returned from `serialize_size`.
  fn unserialize(&mut self, env: &RetroEnvironment, data: *const (), size: usize) -> bool {
    false
  }

  fn cheat_reset(&mut self, env: &RetroEnvironment) {}

  fn cheat_set(&mut self, env: &RetroEnvironment, index: u32, enabled: bool, code: *const libc::c_char) {}

  fn load_game(&mut self, env: &RetroEnvironment, game: Option<RetroGame>) -> RetroLoadGameResult;

  fn load_game_special(&mut self, env: &RetroEnvironment, game_type: u32, info: RetroGame, num_info: usize) -> bool {
    false
  }

  fn unload_game(&mut self, env: &RetroEnvironment) {}

  fn get_memory_data(&mut self, env: &RetroEnvironment, id: u32) -> *mut () {
    std::ptr::null_mut()
  }

  fn get_memory_size(&self, env: &RetroEnvironment, id: u32) -> usize {
    0
  }
}

pub struct RetroAudioInfo {
  sample_rate: f64,
}

impl RetroAudioInfo {
  pub fn new(sample_rate: f64) -> RetroAudioInfo {
    RetroAudioInfo { sample_rate }
  }
}

#[derive(Debug)]
pub enum RetroDevice {
  None = 0,
  Joypad = 1,
  Mouse = 2,
  Keyboard = 3,
  LightGun = 4,
  Analog = 5,
  Pointer = 6,
}

impl From<u32> for RetroDevice {
  fn from(val: u32) -> Self {
    match val {
      0 => Self::None,
      1 => Self::Joypad,
      2 => Self::Mouse,
      3 => Self::Keyboard,
      4 => Self::LightGun,
      5 => Self::Analog,
      6 => Self::Pointer,
      _ => panic!("unrecognized device type. type={}", val),
    }
  }
}

trait Assoc {
  type Type;
}

impl<T> Assoc for Option<T> {
  type Type = T;
}

/// Exposes the `retro_environment_t` callback in an idiomatic fashion. Each of the `RETRO_ENVIRONMENT_*` keys will
/// eventually have a corresponding method here.
///
/// Until that is accomplished, the keys are available in `libretro_rs::sys` and can be used manually with the `get_raw`,
/// `get`, `get_str` and `set_raw` functions.
#[derive(Clone, Copy)]
pub struct RetroEnvironment(<retro_environment_t as Assoc>::Type);

impl RetroEnvironment {
  fn new(cb: <retro_environment_t as Assoc>::Type) -> RetroEnvironment {
    RetroEnvironment(cb)
  }

  /* Commands */

  /// Requests that the frontend shut down. The frontend can refuse to do this, and return false.
  pub fn shutdown(&self) -> bool {
    unsafe { self.cmd_raw(RETRO_ENVIRONMENT_SHUTDOWN) }
  }

  pub fn set_pixel_format(&self, val: RetroPixelFormat) -> bool {
    self.set_u32(RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, val.into())
  }

  pub fn set_support_no_game(&self, val: bool) -> bool {
    self.set_bool(RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME, val)
  }

  /* Queries */

  /// Queries the path where the current libretro core resides.
  pub fn get_libretro_path(&self) -> Option<&str> {
    self.get_str(RETRO_ENVIRONMENT_GET_LIBRETRO_PATH)
  }

  /// Queries the path of the "core assets" directory.
  pub fn get_core_assets_directory(&self) -> Option<&str> {
    self.get_str(RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY)
  }

  /// Queries the path of the save directory.
  pub fn get_save_directory(&self) -> Option<&str> {
    self.get_str(RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY)
  }

  /// Queries the path of the system directory.
  pub fn get_system_directory(&self) -> Option<&str> {
    self.get_str(RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY)
  }

  /// Queries the username associated with the frontend.
  pub fn get_username(&self) -> Option<&str> {
    self.get_str(RETRO_ENVIRONMENT_GET_USERNAME)
  }

  /// Queries a string slice from the environment. A null pointer (`*const c_char`) is interpreted as `None`.
  pub fn get_str<'a>(&'a self, key: u32) -> Option<&'a str> {
    unsafe {
      let s = self.get(key)?;
      CStr::from_ptr(s).to_str().ok()
    }
  }

  /// Queries a struct from the environment. A null pointer (`*const T`) is interpreted as `None`.
  pub unsafe fn get<T>(&self, key: u32) -> Option<*const T> {
    let mut val: *const T = std::ptr::null();
    if self.get_raw(key, &mut val) && !val.is_null() {
      Some(val)
    } else {
      None
    }
  }

  /// Directly invokes the underlying `retro_environment_t` in a "get" fashion.
  #[inline]
  pub unsafe fn get_raw<T>(&self, key: u32, output: *mut *const T) -> bool {
    self.0(key, output as *mut c_void)
  }

  #[inline]
  pub fn set_bool(&self, key: u32, val: bool) -> bool {
    unsafe { self.set_raw(key, &val) }
  }

  #[inline]
  pub fn set_u32(&self, key: u32, val: u32) -> bool {
    unsafe { self.set_raw(key, &val) }
  }

  /// Directly invokes the underlying `retro_environment_t` in a "set" fashion.
  #[inline]
  pub unsafe fn set_raw<T>(&self, key: u32, val: *const T) -> bool {
    self.0(key, val as *mut c_void)
  }

  /// Directly invokes the underlying `retro_environment_t` in a "command" fashion.
  #[inline]
  pub unsafe fn cmd_raw(&self, key: u32) -> bool {
    self.0(key, std::ptr::null_mut())
  }
}

/// Represents the possible ways that a frontend can pass game information to a core.
pub enum RetroGame<'a> {
  /// Used if a core supports "no game" and no game was selected.
  ///
  /// * `meta` contains implementation-specific metadata, if present.
  ///
  /// **Note**: "No game" support is hinted with the `RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME` key.
  None { meta: Option<&'a str> },
  /// Used if a core doesn't need paths, and a game was selected.
  ///
  /// * `meta` contains implementation-specific metadata, if present.
  /// * `data` contains the entire contents of the game.
  Data { meta: Option<&'a str>, data: &'a [u8] },
  /// Used if a core needs paths, and a game was selected.
  ///
  /// * `meta` contains implementation-specific metadata, if present.
  /// * `path` contains the entire absolute path of the game.
  Path { meta: Option<&'a str>, path: &'a str },
}

impl<'a> From<&retro_game_info> for RetroGame<'a> {
  fn from(game: &retro_game_info) -> RetroGame<'a> {
    let meta = if game.meta.is_null() {
      None
    } else {
      unsafe { CStr::from_ptr(game.meta).to_str().ok() }
    };

    if game.path.is_null() && game.data.is_null() {
      return RetroGame::None { meta };
    }

    if !game.data.is_null() {
      unsafe {
        let data = std::slice::from_raw_parts(game.data as *const u8, game.size);
        return RetroGame::Data { meta, data };
      }
    }

    if !game.path.is_null() {
      unsafe {
        let path = CStr::from_ptr(game.path).to_str().unwrap();
        return RetroGame::Path { meta, path };
      }
    }

    unreachable!("`game_info` has a `path` and a `data` field.")
  }
}

pub enum RetroJoypadButton {
  B = 0,
  Y = 1,
  Select = 2,
  Start = 3,
  Up = 4,
  Down = 5,
  Left = 6,
  Right = 7,
  A = 8,
  X = 9,
  L1 = 10,
  R1 = 11,
  L2 = 12,
  R2 = 13,
  L3 = 14,
  R3 = 15,
}

impl Into<u32> for RetroJoypadButton {
  fn into(self) -> u32 {
    match self {
      Self::B => 0,
      Self::Y => 1,
      Self::Select => 2,
      Self::Start => 3,
      Self::Up => 4,
      Self::Down => 5,
      Self::Left => 6,
      Self::Right => 7,
      Self::A => 8,
      Self::X => 9,
      Self::L1 => 10,
      Self::R1 => 11,
      Self::L2 => 12,
      Self::R2 => 13,
      Self::L3 => 14,
      Self::R3 => 15,
    }
  }
}

#[must_use]
pub enum RetroLoadGameResult {
  Failure,
  Success { region: RetroRegion, audio: RetroAudioInfo, video: RetroVideoInfo },
}

/// Represents the set of regions supported by `libretro`.
#[derive(Clone, Copy)]
pub enum RetroRegion {
  /// A 30 frames/second (60 fields/second) video system.
  NTSC = 0,
  /// A 25 frames/second (50 fields/second) video system.
  PAL = 1,
}

impl Into<u32> for RetroRegion {
  fn into(self) -> u32 {
    match self {
      Self::NTSC => 0,
      Self::PAL => 1,
    }
  }
}

#[derive(Clone, Copy)]
pub enum RetroPixelFormat {
  RGB1555,
  XRGB8888,
  RGB565,
}

impl Into<u32> for RetroPixelFormat {
  fn into(self) -> u32 {
    match self {
      RetroPixelFormat::RGB1555 => 0,
      RetroPixelFormat::XRGB8888 => 1,
      RetroPixelFormat::RGB565 => 2,
    }
  }
}

pub struct RetroRuntime {
  audio_sample: <retro_audio_sample_t as Assoc>::Type,
  audio_sample_batch: <retro_audio_sample_batch_t as Assoc>::Type,
  input_state: <retro_input_state_t as Assoc>::Type,
  video_refresh: <retro_video_refresh_t as Assoc>::Type,
}

impl RetroRuntime {
  pub fn new(
    audio_sample: retro_audio_sample_t,
    audio_sample_batch: retro_audio_sample_batch_t,
    input_state: retro_input_state_t,
    video_refresh: retro_video_refresh_t,
  ) -> Option<RetroRuntime> {
    Some(RetroRuntime {
      audio_sample: audio_sample?,
      audio_sample_batch: audio_sample_batch?,
      input_state: input_state?,
      video_refresh: video_refresh?,
    })
  }

  /// Sends audio data to the `libretro` frontend.
  pub fn upload_audio_frame(&self, frame: &[i16]) -> usize {
    unsafe {
      return (self.audio_sample_batch)(frame.as_ptr(), frame.len() / 2);
    }
  }

  /// Sends audio data to the `libretro` frontend.
  pub fn upload_audio_sample(&self, left: i16, right: i16) {
    unsafe {
      return (self.audio_sample)(left, right);
    }
  }

  /// Sends video data to the `libretro` frontend.
  pub fn upload_video_frame(&self, frame: &[u8], width: u32, height: u32, pitch: usize) {
    unsafe {
      return (self.video_refresh)(frame.as_ptr() as *const c_void, width, height, pitch);
    }
  }

  /// Returns true if the specified button is pressed, false otherwise.
  pub fn is_joypad_button_pressed(&self, port: u32, btn: RetroJoypadButton) -> bool {
    unsafe {
      // port, device, index, id
      return (self.input_state)(port, RETRO_DEVICE_JOYPAD, 0, btn.into()) != 0;
    }
  }
}

pub struct RetroSystemInfo {
  name: String,
  version: String,
  valid_extensions: Option<String>,
  block_extract: bool,
  need_full_path: bool,
}

impl RetroSystemInfo {
  pub fn new<N: Into<String>, V: Into<String>>(name: N, version: V) -> RetroSystemInfo {
    RetroSystemInfo {
      name: name.into(),
      version: version.into(),
      valid_extensions: None,
      block_extract: false,
      need_full_path: false,
    }
  }

  pub fn with_valid_extensions(mut self, extensions: &[&str]) -> Self {
    self.valid_extensions = if extensions.len() == 0 {
      None
    } else {
      Some(extensions.join("|"))
    };

    self
  }

  pub fn with_block_extract(mut self) -> Self {
    self.block_extract = true;
    self
  }

  pub fn with_need_full_path(mut self) -> Self {
    self.need_full_path = true;
    self
  }
}

pub struct RetroSystemAvInfo {
  audio: RetroAudioInfo,
  video: RetroVideoInfo,
}

pub struct RetroVideoInfo {
  frame_rate: f64,
  width: u32,
  height: u32,
  aspect_ratio: f32,
  max_width: u32,
  max_height: u32,
  pixel_format: RetroPixelFormat,
}

impl RetroVideoInfo {
  pub fn new(frame_rate: f64, width: u32, height: u32) -> RetroVideoInfo {
    assert_ne!(height, 0);

    RetroVideoInfo {
      frame_rate,
      width,
      height,
      aspect_ratio: (width as f32) / (height as f32),
      max_width: width,
      max_height: height,
      pixel_format: RetroPixelFormat::RGB1555,
    }
  }

  pub fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
    self.aspect_ratio = aspect_ratio;
    self
  }

  pub fn with_max(mut self, width: u32, height: u32) -> Self {
    self.max_width = width;
    self.max_height = height;
    self
  }

  pub fn with_pixel_format(mut self, pixel_format: RetroPixelFormat) -> Self {
    self.pixel_format = pixel_format;
    self
  }
}

/// This is the glue layer between a `RetroCore` implementation, and the `libretro` API.
pub struct RetroInstance<T: RetroCore> {
  pub system: Option<T>,
  pub system_info: Option<RetroSystemInfo>,
  pub system_region: Option<RetroRegion>,
  pub system_av_info: Option<RetroSystemAvInfo>,
  pub audio_sample: retro_audio_sample_t,
  pub audio_sample_batch: retro_audio_sample_batch_t,
  pub environment: Option<RetroEnvironment>,
  pub input_poll: retro_input_poll_t,
  pub input_state: retro_input_state_t,
  pub video_refresh: retro_video_refresh_t,
}

impl<T: RetroCore> RetroInstance<T> {
  /// Invoked by a `libretro` frontend, with the `retro_get_system_info` API call.
  pub fn on_get_system_info(&mut self, info: &mut retro_system_info) {
    let system_info = T::get_system_info();

    info.library_name = system_info.name.as_ptr() as *const c_char;
    info.library_version = system_info.version.as_ptr() as *const c_char;
    info.block_extract = system_info.block_extract;
    info.need_fullpath = system_info.need_full_path;
    info.valid_extensions = match system_info.valid_extensions.as_ref() {
      None => std::ptr::null(),
      Some(ext) => ext.as_ptr() as *const c_char,
    };

    // Hold on to the struct so the pointers don't dangle.
    self.system_info = Some(system_info)
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_system_av_info` API call.
  pub fn on_get_system_av_info(&self, info: &mut retro_system_av_info) {
    let av_info = self
      .system_av_info
      .as_ref()
      .expect("`retro_get_system_av_info` called without a successful `retro_load_game` call. The frontend is not compliant.");

    let audio = &av_info.audio;
    let video = &av_info.video;

    self.environment().set_pixel_format(video.pixel_format);

    info.geometry.aspect_ratio = video.aspect_ratio;
    info.geometry.base_width = video.width;
    info.geometry.base_height = video.height;
    info.geometry.max_width = video.max_width;
    info.geometry.max_height = video.max_height;
    info.timing.fps = video.frame_rate;
    info.timing.sample_rate = audio.sample_rate;
  }

  /// Invoked by a `libretro` frontend, with the `retro_init` API call.
  pub fn on_init(&mut self) {
    let env = self.environment();
    self.system = Some(T::init(&env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_deinit` API call.
  pub fn on_deinit(&mut self) {
    self.system = None;
    self.audio_sample = None;
    self.audio_sample_batch = None;
    self.environment = None;
    self.input_poll = None;
    self.input_state = None;
    self.video_refresh = None;
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_environment` API call.
  pub fn on_set_environment(&mut self, cb: retro_environment_t) {
    self.environment = cb.map(RetroEnvironment::new);
    self.environment().set_support_no_game(T::SUPPORT_NO_GAME);
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_audio_sample` API call.
  pub fn on_set_audio_sample(&mut self, cb: retro_audio_sample_t) {
    self.audio_sample = cb;
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_audio_sample_batch` API call.
  pub fn on_set_audio_sample_batch(&mut self, cb: retro_audio_sample_batch_t) {
    self.audio_sample_batch = cb;
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_input_poll` API call.
  pub fn on_set_input_poll(&mut self, cb: retro_input_poll_t) {
    self.input_poll = cb;
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_input_state` API call.
  pub fn on_set_input_state(&mut self, cb: retro_input_state_t) {
    self.input_state = cb;
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_video_refresh` API call.
  pub fn on_set_video_refresh(&mut self, cb: retro_video_refresh_t) {
    self.video_refresh = cb;
  }

  /// Invoked by a `libretro` frontend, with the `retro_set_controller_port_device` API call.
  pub fn on_set_controller_port_device(&mut self, port: libc::c_uint, device: libc::c_uint) {
    let env = self.environment();
    self.core_mut(|core| core.set_controller_port_device(&env, port, device.into()))
  }

  /// Invoked by a `libretro` frontend, with the `retro_reset` API call.
  pub fn on_reset(&mut self) {
    let env = self.environment();
    self.core_mut(|core| core.reset(&env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_run` API call.
  pub fn on_run(&mut self) {
    unsafe {
      // `input_poll` is required to be called once per `retro_run`.
      (self.input_poll.unwrap())();
    }

    let env = self.environment();

    let runtime = RetroRuntime::new(
      self.audio_sample,
      self.audio_sample_batch,
      self.input_state,
      self.video_refresh,
    )
    .unwrap();

    self.core_mut(|core| core.run(&env, &runtime));
  }

  /// Invoked by a `libretro` frontend, with the `retro_serialize_size` API call.
  pub fn on_serialize_size(&self) -> libc::size_t {
    let env = self.environment();
    self.core_ref(|core| core.serialize_size(&env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_serialize` API call.
  pub fn on_serialize(&self, data: *mut (), size: libc::size_t) -> bool {
    let env = self.environment();
    self.core_ref(|core| core.serialize(&env, data, size))
  }

  /// Invoked by a `libretro` frontend, with the `retro_unserialize` API call.
  pub fn on_unserialize(&mut self, data: *const (), size: libc::size_t) -> bool {
    let env = self.environment();
    self.core_mut(|core| core.unserialize(&env, data, size))
  }

  /// Invoked by a `libretro` frontend, with the `retro_cheat_reset` API call.
  pub fn on_cheat_reset(&mut self) {
    let env = self.environment();
    self.core_mut(|core| core.cheat_reset(&env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_cheat_set` API call.
  pub fn on_cheat_set(&mut self, index: libc::c_uint, enabled: bool, code: *const libc::c_char) {
    let env = self.environment();
    self.core_mut(|core| core.cheat_set(&env, index, enabled, code))
  }

  /// Invoked by a `libretro` frontend, with the `retro_load_game` API call.
  pub fn on_load_game(&mut self, game: Option<&retro_game_info>) -> bool {
    let env = self.environment();

    match self.core_mut(|core| core.load_game(&env, game.map(Into::into))) {
      RetroLoadGameResult::Failure => {
        self.system_av_info = None;
        false
      }
      RetroLoadGameResult::Success { region, audio, video } => {
        self.system_region = Some(region);
        self.system_av_info = Some(RetroSystemAvInfo { audio, video });
        true
      }
    }
  }

  /// Invoked by a `libretro` frontend, with the `retro_load_game_special` API call.
  pub fn on_load_game_special(&mut self, game_type: libc::c_uint, info: &retro_game_info, num_info: libc::size_t) -> bool {
    let env = self.environment();
    self.core_mut(|core| core.load_game_special(&env, game_type, info.into(), num_info))
  }

  /// Invoked by a `libretro` frontend, with the `retro_unload_game` API call.
  pub fn on_unload_game(&mut self) {
    let env = self.environment();
    self.core_mut(|core| core.unload_game(&env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_region` API call.
  pub fn on_get_region(&self) -> libc::c_uint {
    self.system_region.unwrap().into()
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_memory_data` API call.
  pub fn on_get_memory_data(&mut self, id: libc::c_uint) -> *mut () {
    let env = self.environment();
    self.core_mut(|core| core.get_memory_data(&env, id))
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_memory_size` API call.
  pub fn on_get_memory_size(&mut self, id: libc::c_uint) -> libc::size_t {
    let env = self.environment();
    self.core_mut(|core| core.get_memory_size(&env, id))
  }

  #[inline]
  #[doc(hidden)]
  fn environment(&self) -> RetroEnvironment {
    self.environment.unwrap()
  }

  #[inline]
  #[doc(hidden)]
  fn core_mut<F, Output>(&mut self, f: F) -> Output
  where
    F: FnOnce(&mut T) -> Output,
  {
    f(self.system.as_mut().unwrap())
  }

  #[inline]
  #[doc(hidden)]
  fn core_ref<F, Output>(&self, f: F) -> Output
  where
    F: FnOnce(&T) -> Output,
  {
    f(self.system.as_ref().unwrap())
  }
}
