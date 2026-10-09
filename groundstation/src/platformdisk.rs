use std::sync::OnceLock;

use crate::cardwrite::BlockDevice;

pub const PERMISSION_PENDING: i32 = -2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub id: String,
    pub name: String,
}

pub struct Hooks {
    pub disks: fn() -> Vec<Disk>,
    pub open: fn(&str) -> i32,
    pub close: fn(&str),
}

static HOOKS: OnceLock<Hooks> = OnceLock::new();

pub fn install(hooks: Hooks) -> bool {
    HOOKS.set(hooks).is_ok()
}

pub fn installed() -> bool {
    HOOKS.get().is_some()
}

pub fn disks() -> Vec<Disk> {
    HOOKS.get().map(|hooks| (hooks.disks)()).unwrap_or_default()
}

pub struct Card {
    device: Option<Box<dyn BlockDevice + Send>>,
    pub capacity: u64,
    pub label: String,
    pub id: String,
}

impl Card {
    pub fn new(id: &str, label: &str, capacity: u64, device: Box<dyn BlockDevice + Send>) -> Self {
        Self { device: Some(device), capacity, label: label.to_string(), id: id.to_string() }
    }

    pub fn device(&mut self) -> &mut (dyn BlockDevice + Send) {
        self.device.as_deref_mut().expect("a card holds its device until it is dropped")
    }
}

impl Drop for Card {
    fn drop(&mut self) {
        self.device.take();
        if let Some(hooks) = HOOKS.get() {
            (hooks.close)(&self.id);
        }
    }
}

pub enum Opening {
    Ready(Card),
    PermissionPending,
    Failed(String),
}

pub fn open(id: &str) -> Opening {
    match HOOKS.get().map(|hooks| (hooks.open)(id)) {
        None => Opening::Failed("This device cannot write cards".to_string()),
        Some(PERMISSION_PENDING) => Opening::PermissionPending,
        Some(fd) if fd < 0 => Opening::Failed("The card reader could not be opened".to_string()),
        Some(fd) => usb(id, fd),
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
fn usb(id: &str, fd: i32) -> Opening {
    match crate::usbfs::Transport::claim(fd).and_then(crate::usbmsc::UsbDisk::open) {
        Ok(disk) => {
            let identity = disk.identity().clone();
            let label = format!("{} {}", identity.vendor, identity.product).trim().to_string();
            Opening::Ready(Card::new(id, &label, disk.capacity(), Box::new(crate::cardwrite::AlignedDevice::new(disk))))
        }
        Err(e) => {
            if let Some(hooks) = HOOKS.get() {
                (hooks.close)(id);
            }
            Opening::Failed(e.to_string())
        }
    }
}

#[cfg(not(any(target_os = "android", target_os = "linux")))]
fn usb(id: &str, _fd: i32) -> Opening {
    if let Some(hooks) = HOOKS.get() {
        (hooks.close)(id);
    }
    Opening::Failed("USB card readers are not supported on this platform yet".to_string())
}
