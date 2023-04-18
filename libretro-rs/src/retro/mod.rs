extern crate core;

mod av_info;
mod convert;
pub mod core_macro;
mod environment;
mod extensions;
mod logger;
mod memory;
mod system_info;

pub use av_info::*;
pub use convert::*;
pub use core_macro::*;
pub use environment::*;
pub use extensions::*;
pub use logger::*;
pub use memory::*;
pub use system_info::*;

pub use LoadGameResult::*;

use crate::ffi::*;
use c_utf8::CUtf8;
use core::ffi::*;
use core::ops::*;

#[allow(unused_variables)]
pub trait Core: Sized {
  /// Called during `retro_set_environment`.
  fn set_environment(env: &mut impl SetEnvironmentEnvironment) {}

  /// Called during `retro_init`. This function is provided for the sake of completeness; it's generally redundant
  /// with [`Core::load_game`].
  fn init(env: &mut impl InitEnvironment) {}

  /// Called to get information about the core. This information can then be displayed in a frontend, or used to
  /// construct core-specific paths.
  fn get_system_info() -> SystemInfo;

  fn get_system_av_info(&self, env: &mut impl GetSystemAvInfoEnvironment) -> SystemAVInfo;

  fn get_region(&self, env: &mut impl GetRegionEnvironment) -> Region {
    Region::NTSC
  }

  /// Called to associate a particular device with a particular port. A core is allowed to ignore this request.
  fn set_controller_port_device(&mut self, env: &mut impl SetPortDeviceEnvironment, port: DevicePort, device: Device) {}

  /// Called when a player resets their game.
  fn reset(&mut self, env: &mut impl ResetEnvironment);

  /// Called continuously once the core is initialized and a game is loaded. The core is expected to advance emulation
  /// by a single frame before returning.
  fn run(&mut self, env: &mut impl RunEnvironment, runtime: &Runtime);

  /// Called to determine the size of the save state buffer. This is only ever called once per run, and the core must
  /// not exceed the size returned here for subsequent saves.
  fn serialize_size(&self, env: &mut impl SerializeSizeEnvironment) -> usize {
    0
  }

  /// Allows a core to save its internal state into the specified buffer. The buffer is guaranteed to be at least `size`
  /// bytes, where `size` is the value returned from `serialize_size`.
  fn serialize(&self, env: &mut impl SerializeEnvironment, data: &mut [u8]) -> bool {
    false
  }

  /// Allows a core to load its internal state from the specified buffer. The buffer is guaranteed to be at least `size`
  /// bytes, where `size` is the value returned from `serialize_size`.
  fn unserialize(&mut self, env: &mut impl UnserializeEnvironment, data: &[u8]) -> bool {
    false
  }

  fn cheat_reset(&mut self, env: &mut impl CheatResetEnvironment) {}

  fn cheat_set(&mut self, env: &mut impl CheatSetEnvironment, index: u32, enabled: bool, code: &str) {}

  /// Called when a new instance of the core is needed. The `env` parameter can be used to set-up and/or query values
  /// required for the core to function properly.
  fn load_game(env: &mut impl LoadGameEnvironment, game: Game) -> LoadGameResult<Self>;

  fn load_game_special(env: &mut impl LoadGameSpecialEnvironment, game_type: GameType, info: Game) -> LoadGameResult<Self> {
    Failure
  }

  fn unload_game(&mut self, env: &mut impl UnloadGameEnvironment) {}

  fn get_memory_data(&mut self, env: &mut impl GetMemoryDataEnvironment, id: MemoryType) -> Option<&mut [u8]> {
    None
  }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameType(u32);

impl GameType {
  pub fn new(n: u32) -> Self {
    Self(n)
  }

  pub fn into_inner(self) -> u32 {
    self.0
  }
}

impl From<u32> for GameType {
  fn from(n: u32) -> Self {
    Self(n)
  }
}

impl From<GameType> for u32 {
  fn from(game_type: GameType) -> Self {
    game_type.into_inner()
  }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemoryType(u32);

impl MemoryType {
  pub fn new(n: u32) -> Self {
    Self(n)
  }

  pub fn into_inner(self) -> u32 {
    self.0
  }
}

impl From<u32> for MemoryType {
  fn from(n: u32) -> Self {
    Self(n)
  }
}

impl From<MemoryType> for u32 {
  fn from(memory_type: MemoryType) -> Self {
    memory_type.into_inner()
  }
}

trait TypeId: Sized {
  fn into_discriminant(self) -> u8;
  fn from_discriminant(id: u8) -> Option<Self>;
}

impl TypeId for () {
  fn into_discriminant(self) -> u8 {
    0
  }

  fn from_discriminant(_id: u8) -> Option<Self> {
    None
  }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Device {
  #[default]
  None = 0,
  Joypad = 1,
  Mouse = 2,
  Keyboard = 3,
  LightGun = 4,
  Analog = 5,
  Pointer = 6,
}

impl TryFrom<c_uint> for Device {
  type Error = ();

  fn try_from(val: c_uint) -> Result<Self, Self::Error> {
    match val {
      0 => Ok(Self::None),
      1 => Ok(Self::Joypad),
      2 => Ok(Self::Mouse),
      3 => Ok(Self::Keyboard),
      4 => Ok(Self::LightGun),
      5 => Ok(Self::Analog),
      6 => Ok(Self::Pointer),
      _ => Err(()),
    }
  }
}

/// A libretro device port.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DevicePort(u8);

impl DevicePort {
  /// Creates a [`DevicePort`].
  pub fn new(port_number: u8) -> Self {
    DevicePort(port_number)
  }

  // Converts this port back into a u8.
  pub fn into_inner(self) -> u8 {
    self.0
  }
}

impl From<u8> for DevicePort {
  fn from(port_number: u8) -> Self {
    Self::new(port_number)
  }
}

impl From<DevicePort> for u8 {
  fn from(port: DevicePort) -> Self {
    port.into_inner()
  }
}

/// Represents the possible ways that a frontend can pass game information to a core.
#[derive(Debug, Clone)]
pub enum Game<'a> {
  /// Used if a core supports "no game" and no game was selected.
  ///
  /// * `meta` contains implementation-specific metadata, if present.
  ///
  /// **Note**: "No game" support is hinted with the `RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME` key.
  None { meta: Option<&'a CStr> },
  /// Used if a core doesn't need paths, and a game was selected.
  ///
  /// * `meta` contains implementation-specific metadata, if present.
  /// * `data` contains the entire contents of the game.
  Data { meta: Option<&'a CStr>, data: &'a [u8] },
  /// Used if a core needs paths, and a game was selected.
  ///
  /// * `meta` contains implementation-specific metadata, if present.
  /// * `path` contains the entire absolute path of the game.
  Path { meta: Option<&'a CStr>, path: &'a CUtf8 },
}

impl<'a> From<Option<&retro_game_info>> for Game<'a> {
  fn from(info: Option<&retro_game_info>) -> Self {
    match info {
      None => Game::None { meta: None },
      Some(info) => Game::from(info),
    }
  }
}

impl<'a> Default for Game<'a> {
  fn default() -> Self {
    Game::None { meta: None }
  }
}

impl<'a> From<&retro_game_info> for Game<'a> {
  fn from(game: &retro_game_info) -> Game<'a> {
    let meta = unsafe { game.meta.as_ref().map(|x| CStr::from_ptr(x)) };

    match (game.path.is_null(), game.data.is_null()) {
      (true, true) => Game::None { meta },
      (_, false) => unsafe {
        let data = core::slice::from_raw_parts(game.data as *const u8, game.size);
        Game::Data { meta, data }
      },
      (false, _) => unsafe {
        let path = CUtf8::from_c_str_unchecked(CStr::from_ptr(game.path));
        Game::Path { meta, path }
      },
    }
  }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum JoypadButton {
  #[default]
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
  #[cfg(experimental)]
  Mask = 256,
}

impl From<JoypadButton> for u32 {
  fn from(button: JoypadButton) -> u32 {
    match button {
      JoypadButton::B => 0,
      JoypadButton::Y => 1,
      JoypadButton::Select => 2,
      JoypadButton::Start => 3,
      JoypadButton::Up => 4,
      JoypadButton::Down => 5,
      JoypadButton::Left => 6,
      JoypadButton::Right => 7,
      JoypadButton::A => 8,
      JoypadButton::X => 9,
      JoypadButton::L1 => 10,
      JoypadButton::R1 => 11,
      JoypadButton::L2 => 12,
      JoypadButton::R2 => 13,
      JoypadButton::L3 => 14,
      JoypadButton::R3 => 15,
      #[cfg(experimental)]
      JoypadButton::Mask => 256,
    }
  }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadGameResult<T> {
  Failure,
  Success(T),
}

impl<T> From<LoadGameResult<T>> for Option<T>
where
  T: Core,
{
  fn from(result: LoadGameResult<T>) -> Self {
    match result {
      Failure => None,
      Success(core) => Some(core),
    }
  }
}

impl<T> From<Option<T>> for LoadGameResult<T>
where
  T: Core,
{
  fn from(option: Option<T>) -> Self {
    match option {
      None => Failure,
      Some(core) => Success(core),
    }
  }
}

impl<T, E> From<Result<T, E>> for LoadGameResult<T>
where
  T: Core,
{
  fn from(result: Result<T, E>) -> Self {
    match result {
      Err(_) => Failure,
      Ok(core) => Success(core),
    }
  }
}

impl<T> From<LoadGameResult<T>> for Result<T, ()>
where
  T: Core,
{
  fn from(result: LoadGameResult<T>) -> Self {
    match result {
      Failure => Err(()),
      Success(core) => Ok(core),
    }
  }
}

/// Represents the set of regions supported by `libretro`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Region {
  /// A 30 frames/second (60 fields/second) video system.
  #[default]
  NTSC = 0,
  /// A 25 frames/second (50 fields/second) video system.
  PAL = 1,
}

impl From<Region> for c_uint {
  fn from(region: Region) -> Self {
    match region {
      Region::NTSC => 0,
      Region::PAL => 1,
    }
  }
}

#[repr(i32)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PixelFormat {
  #[default]
  RGB1555 = 0,
  XRGB8888 = 1,
  RGB565 = 2,
}

pub struct Runtime {
  audio_sample: retro_audio_sample_t,
  audio_sample_batch: retro_audio_sample_batch_t,
  input_state: retro_input_state_t,
  video_refresh: retro_video_refresh_t,
}

impl Runtime {
  pub fn new(
    audio_sample: retro_audio_sample_t,
    audio_sample_batch: retro_audio_sample_batch_t,
    input_state: retro_input_state_t,
    video_refresh: retro_video_refresh_t,
  ) -> Runtime {
    Runtime {
      audio_sample,
      audio_sample_batch,
      input_state,
      video_refresh,
    }
  }

  /// Sends audio data to the `libretro` frontend.
  pub fn upload_audio_frame(&self, frame: &[i16]) -> usize {
    let cb = self
      .audio_sample_batch
      .expect("`upload_audio_frame` called without registering an `audio_sample_batch` callback");

    unsafe { cb(frame.as_ptr(), frame.len() / 2) }
  }

  /// Sends audio data to the `libretro` frontend.
  pub fn upload_audio_sample(&self, left: i16, right: i16) {
    let cb = self
      .audio_sample
      .expect("`upload_audio_sample` called without registering an `audio_sample` callback");

    unsafe { cb(left, right) }
  }

  /// Sends video data to the `libretro` frontend.
  pub fn upload_video_frame(&self, frame: &[u8], width: u32, height: u32, pitch: usize) {
    let cb = self
      .video_refresh
      .expect("`upload_video_frame` called without registering a `video_refresh` callback");

    unsafe { cb(frame.as_ptr() as *const c_void, width, height, pitch) }
  }

  /// Returns true if the specified button is pressed, false otherwise.
  pub fn is_joypad_button_pressed(&self, port: DevicePort, btn: JoypadButton) -> bool {
    let cb = self
      .input_state
      .expect("`is_joypad_button_pressed` called without registering an `input_state` callback");

    let port = c_uint::from(port.into_inner());
    let device = RETRO_DEVICE_JOYPAD;
    let index = 0;
    let id = btn.into();
    unsafe { cb(port, device, index, id) != 0 }
  }
}

/// This is the glue layer between a [Core] and the `libretro` API.
pub struct Instance<T> {
  pub system: Option<T>,
  pub audio_sample: retro_audio_sample_t,
  pub audio_sample_batch: retro_audio_sample_batch_t,
  pub environment: retro_environment_t,
  pub input_poll: retro_input_poll_t,
  pub input_state: retro_input_state_t,
  pub video_refresh: retro_video_refresh_t,
}

impl<T: Core> Instance<T> {
  /// Invoked by a `libretro` frontend, with the `retro_get_system_info` API call.
  pub fn on_get_system_info(&mut self, info: &mut retro_system_info) {
    *info = T::get_system_info().into();
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_system_av_info` API call.
  pub fn on_get_system_av_info(&self, info: &mut retro_system_av_info) {
    let system = self
      .system
      .as_ref()
      .expect("`retro_get_system_av_info` called without a successful `retro_load_game` call. The frontend is not compliant");
    *info = system.get_system_av_info(&mut self.environment()).into();
  }

  /// Invoked by a `libretro` frontend, with the `retro_init` API call.
  pub fn on_init(&self) {
    T::init(&mut self.environment());
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
  pub fn on_set_environment(&mut self, mut env: EnvironmentCallback) {
    T::set_environment(&mut env);

    self.environment = Some(env);
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
  pub fn on_set_controller_port_device(&mut self, port: c_uint, device: c_uint) {
    if let Ok(device) = device.try_into() {
      if let Ok(port_num) = u8::try_from(port) {
        let mut env = self.environment();
        let port = DevicePort(port_num);
        self.core_mut(|core| core.set_controller_port_device(&mut env, port, device))
      }
    }
  }

  /// Invoked by a `libretro` frontend, with the `retro_reset` API call.
  pub fn on_reset(&mut self) {
    let mut env = self.environment();
    self.core_mut(|core| core.reset(&mut env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_run` API call.
  pub fn on_run(&mut self) {
    // `input_poll` is required to be called once per `retro_run`.
    self.input_poll();

    let mut env = self.environment();

    let runtime = Runtime::new(
      self.audio_sample,
      self.audio_sample_batch,
      self.input_state,
      self.video_refresh,
    );

    self.core_mut(|core| core.run(&mut env, &runtime));
  }

  fn input_poll(&mut self) {
    let cb = self
      .input_poll
      .expect("`on_run` called without registering an `input_poll` callback");

    unsafe { cb() }
  }

  /// Invoked by a `libretro` frontend, with the `retro_serialize_size` API call.
  pub fn on_serialize_size(&self) -> usize {
    let mut env = self.environment();
    self.core_ref(|core| core.serialize_size(&mut env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_serialize` API call.
  pub fn on_serialize(&self, data: *mut (), size: usize) -> bool {
    unsafe {
      let data = core::slice::from_raw_parts_mut(data as *mut u8, size);
      let mut env = self.environment();
      self.core_ref(|core| core.serialize(&mut env, data))
    }
  }

  /// Invoked by a `libretro` frontend, with the `retro_unserialize` API call.
  pub fn on_unserialize(&mut self, data: *const (), size: usize) -> bool {
    unsafe {
      let data = core::slice::from_raw_parts(data as *const u8, size);
      let mut env = self.environment();
      self.core_mut(|core| core.unserialize(&mut env, data))
    }
  }

  /// Invoked by a `libretro` frontend, with the `retro_cheat_reset` API call.
  pub fn on_cheat_reset(&mut self) {
    let mut env = self.environment();
    self.core_mut(|core| core.cheat_reset(&mut env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_cheat_set` API call.
  ///
  /// # Safety
  /// `code` must be a valid argument to [`CStr::from_ptr`].
  pub unsafe fn on_cheat_set(&mut self, index: c_uint, enabled: bool, code: *const c_char) {
    unsafe {
      let code = CStr::from_ptr(code).to_str().expect("`code` contains invalid data");
      let mut env = self.environment();
      self.core_mut(|core| core.cheat_set(&mut env, index, enabled, code))
    }
  }

  /// Invoked by a `libretro` frontend, with the `retro_load_game` API call.
  ///
  /// # Safety
  /// `game` must remain valid until [`Instance::on_unload_game`] is called.
  pub unsafe fn on_load_game(&mut self, game: *const retro_game_info) -> bool {
    let mut env = self.environment();
    let game = game.as_ref().map_or_else(Game::default, Game::from);
    self.system = T::load_game(&mut env, game).into();
    self.system.is_some()
  }

  /// Invoked by a `libretro` frontend, with the `retro_load_game_special` API call.
  pub fn on_load_game_special(&mut self, game_type: GameType, info: &retro_game_info, _num_info: usize) -> bool {
    let mut env = self.environment();
    self.system = T::load_game_special(&mut env, game_type, info.into()).into();
    self.system.is_some()
  }

  /// Invoked by a `libretro` frontend, with the `retro_unload_game` API call.
  pub fn on_unload_game(&mut self) {
    let mut env = self.environment();
    self.core_mut(|core| core.unload_game(&mut env))
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_region` API call.
  pub fn on_get_region(&self) -> c_uint {
    let system = self.system.as_ref().expect("`on_get_region` called without a game loaded.");
    c_uint::from(system.get_region(&mut self.environment()))
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_memory_data` API call.
  pub fn on_get_memory_data(&mut self, id: MemoryType) -> *mut () {
    let mut env = self.environment();
    self.core_mut(|core| {
      core
        .get_memory_data(&mut env, id)
        .map_or_else(std::ptr::null_mut, |data| data.as_mut_ptr() as *mut ())
    })
  }

  /// Invoked by a `libretro` frontend, with the `retro_get_memory_size` API call.
  pub fn on_get_memory_size(&mut self, id: MemoryType) -> usize {
    let mut env = self.environment();
    self.core_mut(|core| core.get_memory_data(&mut env, id).map_or(0, |data| data.len()))
  }

  #[inline]
  #[doc(hidden)]
  fn environment(&self) -> EnvironmentCallback {
    self.environment.expect("unable to retrieve the environment callback")
  }

  #[inline]
  #[doc(hidden)]
  fn core_mut<F, Output>(&mut self, f: F) -> Output
  where
    F: FnOnce(&mut T) -> Output,
  {
    let sys = self
      .system
      .as_mut()
      .expect("`core_mut` called when no system has been created");

    f(sys)
  }

  #[inline]
  #[doc(hidden)]
  fn core_ref<F, Output>(&self, f: F) -> Output
  where
    F: FnOnce(&T) -> Output,
  {
    let sys = self
      .system
      .as_ref()
      .expect("`core_ref` called when no system has been created");

    f(sys)
  }
}