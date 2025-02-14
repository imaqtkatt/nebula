#[derive(Clone, Copy, Debug)]
pub enum Device {
  Macos,
  Windows,
  Linux,
  Unknown,
}

pub const DEVICE: Device = if cfg!(target_os = "macos") {
  Device::Macos
} else if cfg!(target_os = "windows") {
  Device::Windows
} else if cfg!(target_os = "linux") {
  Device::Linux
} else {
  Device::Unknown
};

impl Device {
  pub const TAG_MACOS: u8 = 0x1;
  pub const TAG_WINDOWS: u8 = 0x2;
  pub const TAG_LINUX: u8 = 0x3;
  pub const TAG_UNKNOWN: u8 = 0x4;
}
