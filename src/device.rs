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
