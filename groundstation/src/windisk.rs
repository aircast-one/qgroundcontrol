use std::io::{self, Read, Seek, SeekFrom, Write};

use crate::cardwrite::BlockDevice;
use crate::platformdisk::Disk;

pub const WINDISK_NEEDS_ADMIN: &str = "Writing a card needs administrator rights";
pub const WINDISK_VIRTUAL_ENV: &str = "QGC_FLASHER_VIRTUAL_DISKS";
const WINDISK_IN_USE: &str = "Another program is using the card. Close it and try again.";
const WINDISK_WRITE_PROTECTED: &str =
    "The card is write-protected. Slide the lock switch on its side up and try again.";
const WINDISK_NOT_CONNECTED: &str = "That card reader is not connected";
const WINDISK_UNNAMED: &str = "Card reader";
const WINDISK_PATH_PREFIX: &str = r"\\.\PhysicalDrive";
const WINDISK_NUMBER_LIMIT: u32 = 64;
const WINDISK_BUS_USB: i32 = 7;
const WINDISK_BUS_SD: i32 = 12;
const WINDISK_BUS_MMC: i32 = 13;
const WINDISK_BUS_FILE_BACKED_VIRTUAL: i32 = 15;
const WINDISK_ACCESS_DENIED: i32 = 5;
const WINDISK_FILE_NOT_FOUND: i32 = 2;
const WINDISK_PATH_NOT_FOUND: i32 = 3;
const WINDISK_SHARING_VIOLATION: i32 = 32;
const WINDISK_WRITE_PROTECT: i32 = 19;
const WINDISK_EXTENTS_HEADER: usize = 8;
const WINDISK_EXTENT_LEN: usize = 24;
const WINDISK_DESCRIPTOR_VENDOR: usize = 12;
const WINDISK_DESCRIPTOR_PRODUCT: usize = 16;
const WINDISK_DESCRIPTOR_BUS: usize = 28;
const WINDISK_GEOMETRY_SIZE: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub number: u32,
    pub bus: i32,
    pub vendor: String,
    pub product: String,
    pub size: u64,
}

pub fn disk_path(number: u32) -> String {
    format!("{WINDISK_PATH_PREFIX}{number}")
}

pub fn disk_number(id: &str) -> Option<u32> {
    id.strip_prefix(WINDISK_PATH_PREFIX)?.parse().ok()
}

pub fn virtual_disks_allowed(value: Option<&str>) -> bool {
    value == Some("1")
}

pub fn writable_bus(bus: i32, virtual_disks: bool) -> bool {
    matches!(bus, WINDISK_BUS_USB | WINDISK_BUS_SD | WINDISK_BUS_MMC)
        || (virtual_disks && bus == WINDISK_BUS_FILE_BACKED_VIRTUAL)
}

pub fn label(probe: &Probe) -> String {
    Some(
        format!("{} {}", probe.vendor.trim(), probe.product.trim())
            .trim()
            .to_string(),
    )
    .filter(|label| !label.is_empty())
    .unwrap_or_else(|| WINDISK_UNNAMED.to_string())
}

pub fn cards(probes: &[Probe], system: &[u32], virtual_disks: bool) -> Vec<Disk> {
    probes
        .iter()
        .filter(|probe| {
            probe.size > 0
                && !system.contains(&probe.number)
                && writable_bus(probe.bus, virtual_disks)
        })
        .map(|probe| Disk {
            id: disk_path(probe.number),
            name: format!(
                "{} · {}",
                label(probe),
                crate::flasher::size_text(probe.size)
            ),
        })
        .collect()
}

pub fn refusal(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(WINDISK_ACCESS_DENIED) => WINDISK_NEEDS_ADMIN.to_string(),
        Some(WINDISK_SHARING_VIOLATION) => WINDISK_IN_USE.to_string(),
        Some(WINDISK_WRITE_PROTECT) => WINDISK_WRITE_PROTECTED.to_string(),
        Some(WINDISK_FILE_NOT_FOUND | WINDISK_PATH_NOT_FOUND) => WINDISK_NOT_CONNECTED.to_string(),
        _ => format!("The card could not be opened: {error}"),
    }
}

pub fn ascii_at(bytes: &[u8], offset: usize) -> String {
    match offset {
        0 => String::new(),
        at => bytes
            .get(at..)
            .map(|tail| {
                tail.iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect::<String>()
                    .trim()
                    .to_string()
            })
            .unwrap_or_default(),
    }
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn extent_disks(bytes: &[u8]) -> Vec<u32> {
    (0..word(bytes, 0).unwrap_or(0) as usize)
        .map_while(|index| word(bytes, WINDISK_EXTENTS_HEADER + index * WINDISK_EXTENT_LEN))
        .collect()
}

pub fn geometry_size(bytes: &[u8]) -> u64 {
    bytes
        .get(WINDISK_GEOMETRY_SIZE..WINDISK_GEOMETRY_SIZE + 8)
        .and_then(|b| b.try_into().ok())
        .map(i64::from_le_bytes)
        .and_then(|size| u64::try_from(size).ok())
        .unwrap_or(0)
}

pub fn described(number: u32, descriptor: &[u8], size: u64) -> Option<Probe> {
    let text = |field: usize| word(descriptor, field).map(|at| ascii_at(descriptor, at as usize));
    Some(Probe {
        number,
        bus: word(descriptor, WINDISK_DESCRIPTOR_BUS)? as i32,
        vendor: text(WINDISK_DESCRIPTOR_VENDOR)?,
        product: text(WINDISK_DESCRIPTOR_PRODUCT)?,
        size,
    })
}

pub type Claim<F> = Box<dyn FnOnce() -> io::Result<Vec<F>> + Send>;

pub struct RawDisk<F> {
    file: F,
    len: u64,
    pos: u64,
    claim: Option<Claim<F>>,
    _locks: Vec<F>,
}

impl<F: Read + Write + Seek> RawDisk<F> {
    pub fn new(file: F, len: u64, claim: Claim<F>) -> Self {
        Self {
            file,
            len,
            pos: 0,
            claim: Some(claim),
            _locks: Vec::new(),
        }
    }
}

impl<F> RawDisk<F> {
    fn claimed(&mut self) -> io::Result<()> {
        match self.claim.take() {
            None => Ok(()),
            Some(claim) => claim()
                .map(|locks| self._locks = locks)
                .inspect_err(|refused| {
                    let reason = refused.to_string();
                    self.claim = Some(Box::new(move || Err(io::Error::other(reason))));
                }),
        }
    }
}

impl<F: Read> Read for RawDisk<F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let want = self.len.saturating_sub(self.pos).min(buf.len() as u64) as usize;
        match want {
            0 => Ok(0),
            want => {
                let got = self.file.read(&mut buf[..want])?;
                self.pos += got as u64;
                Ok(got)
            }
        }
    }
}

impl<F: Write> Write for RawDisk<F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.claimed()?;
        match self.pos + buf.len() as u64 > self.len {
            true => Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "the image does not fit on the card",
            )),
            false => {
                let wrote = self.file.write(buf)?;
                self.pos += wrote as u64;
                Ok(wrote)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl<F: Seek> Seek for RawDisk<F> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(delta) => i128::from(self.pos) + i128::from(delta),
            SeekFrom::End(delta) => i128::from(self.len) + i128::from(delta),
        };
        match u64::try_from(target) {
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek to a negative position",
            )),
            Ok(at) => {
                self.pos = self.file.seek(SeekFrom::Start(at))?;
                Ok(self.pos)
            }
        }
    }
}

impl BlockDevice for RawDisk<std::fs::File> {
    fn sync(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }
}

#[cfg(windows)]
pub use sys::{disks, open};

#[cfg(windows)]
mod sys {
    use std::ffi::c_void;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_SHARE_READ, FILE_SHARE_WRITE, FindFirstVolumeW, FindNextVolumeW, FindVolumeClose,
        GetVolumeNameForVolumeMountPointW, GetVolumePathNameW,
        IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
        IOCTL_DISK_IS_WRITABLE, IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery,
        StorageDeviceProperty,
    };

    use super::*;
    use crate::cardwrite::AlignedDevice;
    use crate::platformdisk::{Card, Opening};

    const WINDISK_NAME_UNITS: usize = 1024;
    const WINDISK_DESCRIPTOR_BYTES: usize = 1024;
    const WINDISK_GEOMETRY_BYTES: usize = 256;
    const WINDISK_EXTENT_SLOTS: usize = 32;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn narrow(units: &[u16]) -> String {
        String::from_utf16_lossy(
            &units[..units.iter().position(|&u| u == 0).unwrap_or(units.len())],
        )
    }

    fn shared(options: &mut OpenOptions) -> &mut OpenOptions {
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
    }

    fn query(path: &str) -> io::Result<File> {
        shared(OpenOptions::new().access_mode(0)).open(path)
    }

    fn writable(path: &str) -> io::Result<File> {
        shared(OpenOptions::new().read(true).write(true)).open(path)
    }

    fn ioctl(file: &File, code: u32, input: &[u8], output: &mut [u8]) -> io::Result<usize> {
        let mut returned = 0u32;
        let done = unsafe {
            DeviceIoControl(
                file.as_raw_handle() as _,
                code,
                input.as_ptr() as *const c_void,
                input.len() as u32,
                output.as_mut_ptr() as *mut c_void,
                output.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        match done {
            0 => Err(io::Error::last_os_error()),
            _ => Ok(returned as usize),
        }
    }

    fn probe(number: u32) -> Option<Probe> {
        let file = query(&disk_path(number)).ok()?;
        let asked = [
            StorageDeviceProperty.to_le_bytes(),
            PropertyStandardQuery.to_le_bytes(),
            [0u8; 4],
        ]
        .concat();
        let mut descriptor = vec![0u8; WINDISK_DESCRIPTOR_BYTES];
        let got = ioctl(&file, IOCTL_STORAGE_QUERY_PROPERTY, &asked, &mut descriptor).ok()?;
        let mut geometry = vec![0u8; WINDISK_GEOMETRY_BYTES];
        let size = ioctl(&file, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, &[], &mut geometry)
            .map(|len| geometry_size(&geometry[..len]))
            .unwrap_or(0);
        described(number, &descriptor[..got], size)
    }

    fn volume_of(path: &Path) -> Option<String> {
        let mut root = vec![0u16; WINDISK_NAME_UNITS];
        let mut name = vec![0u16; WINDISK_NAME_UNITS];
        let asked = wide(&path.to_string_lossy());
        let rooted =
            unsafe { GetVolumePathNameW(asked.as_ptr(), root.as_mut_ptr(), root.len() as u32) }
                != 0;
        let named = rooted
            && unsafe {
                GetVolumeNameForVolumeMountPointW(
                    root.as_ptr(),
                    name.as_mut_ptr(),
                    name.len() as u32,
                )
            } != 0;
        named.then(|| narrow(&name))
    }

    fn disks_of(volume: &str) -> Vec<u32> {
        let mut extents =
            vec![0u8; WINDISK_EXTENTS_HEADER + WINDISK_EXTENT_LEN * WINDISK_EXTENT_SLOTS];
        query(volume.trim_end_matches('\\'))
            .and_then(|file| {
                ioctl(
                    &file,
                    IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                    &[],
                    &mut extents,
                )
            })
            .map(|got| extent_disks(&extents[..got]))
            .unwrap_or_default()
    }

    fn volumes() -> Vec<String> {
        let mut name = vec![0u16; WINDISK_NAME_UNITS];
        let find = unsafe { FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32) };
        match find == INVALID_HANDLE_VALUE {
            true => Vec::new(),
            false => {
                let first = narrow(&name);
                let found: Vec<String> = std::iter::once(first)
                    .chain(std::iter::from_fn(|| {
                        (unsafe { FindNextVolumeW(find, name.as_mut_ptr(), name.len() as u32) }
                            != 0)
                            .then(|| narrow(&name))
                    }))
                    .collect();
                unsafe { FindVolumeClose(find) };
                found
            }
        }
    }

    fn system_disks() -> Option<Vec<u32>> {
        let windows = std::env::var_os("SystemRoot").map(PathBuf::from)?;
        let system = Some(disks_of(&volume_of(&windows)?)).filter(|disks| !disks.is_empty())?;
        let running = std::env::current_exe()
            .ok()
            .and_then(|exe| volume_of(&exe))
            .map(|volume| disks_of(&volume))
            .unwrap_or_default();
        Some(system.into_iter().chain(running).collect())
    }

    fn eligible() -> Vec<(Disk, Probe)> {
        let virtual_disks =
            virtual_disks_allowed(std::env::var(WINDISK_VIRTUAL_ENV).ok().as_deref());
        let Some(system) = system_disks() else {
            return Vec::new();
        };
        let probes: Vec<Probe> = (0..WINDISK_NUMBER_LIMIT).filter_map(probe).collect();
        cards(&probes, &system, virtual_disks)
            .into_iter()
            .filter_map(|disk| {
                let number = disk_number(&disk.id)?;
                let found = probes.iter().find(|p| p.number == number)?.clone();
                Some((disk, found))
            })
            .collect()
    }

    pub fn disks() -> Vec<Disk> {
        eligible().into_iter().map(|(disk, _)| disk).collect()
    }

    fn hold(volume: &str) -> io::Result<File> {
        let file =
            writable(volume.trim_end_matches('\\')).map_err(|e| io::Error::other(refusal(&e)))?;
        let locked = ioctl(&file, FSCTL_LOCK_VOLUME, &[], &mut []).is_ok();
        let dismounted = ioctl(&file, FSCTL_DISMOUNT_VOLUME, &[], &mut []).is_ok();
        match dismounted && (locked || ioctl(&file, FSCTL_LOCK_VOLUME, &[], &mut []).is_ok()) {
            true => Ok(file),
            false => Err(io::Error::other(WINDISK_IN_USE)),
        }
    }

    fn take_volumes(number: u32) -> io::Result<Vec<File>> {
        volumes()
            .iter()
            .filter(|volume| disks_of(volume).contains(&number))
            .map(|volume| hold(volume))
            .collect()
    }

    fn claim(probe: &Probe) -> Result<RawDisk<File>, String> {
        let file = writable(&disk_path(probe.number)).map_err(|e| refusal(&e))?;
        ioctl(&file, IOCTL_DISK_IS_WRITABLE, &[], &mut []).map_err(|e| refusal(&e))?;
        let number = probe.number;
        Ok(RawDisk::new(
            file,
            probe.size,
            Box::new(move || take_volumes(number)),
        ))
    }

    pub fn open(id: &str) -> Opening {
        match eligible().into_iter().find(|(disk, _)| disk.id == id) {
            None => Opening::Failed(WINDISK_NOT_CONNECTED.to_string()),
            Some((_, probe)) => match claim(&probe) {
                Ok(device) => Opening::Ready(Card::new(
                    id,
                    &label(&probe),
                    probe.size,
                    Box::new(AlignedDevice::new(device)),
                )),
                Err(reason) => Opening::Failed(reason),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardwrite::tests::{build_image, cloud_init_config, read_fat_file};
    use crate::cardwrite::{self, AlignedDevice, Params, Stage};

    fn probe(number: u32, bus: i32, size: u64) -> Probe {
        Probe {
            number,
            bus,
            vendor: "Generic".into(),
            product: "STORAGE DEVICE".into(),
            size,
        }
    }

    fn unclaimed() -> Claim<std::fs::File> {
        Box::new(|| Ok(Vec::new()))
    }

    #[test]
    fn the_card_is_taken_from_windows_on_the_first_write_not_on_opening() {
        let (path, file) = card_file("claim", 8192);
        let taken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = taken.clone();
        let mut disk = RawDisk::new(
            file,
            8192,
            Box::new(move || {
                counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(Vec::new())
            }),
        );
        disk.read(&mut [0u8; 4096]).unwrap();
        disk.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(taken.load(std::sync::atomic::Ordering::Relaxed), 0);
        disk.write(&[1u8; 4096]).unwrap();
        disk.write(&[2u8; 4096]).unwrap();
        assert_eq!(taken.load(std::sync::atomic::Ordering::Relaxed), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_card_windows_will_not_let_go_refuses_every_write() {
        let (path, file) = card_file("busy", 8192);
        let mut disk = RawDisk::new(
            file,
            8192,
            Box::new(|| Err(io::Error::other(WINDISK_IN_USE))),
        );
        assert_eq!(
            disk.write(&[1u8; 512]).unwrap_err().to_string(),
            WINDISK_IN_USE
        );
        assert_eq!(
            disk.write(&[1u8; 512]).unwrap_err().to_string(),
            WINDISK_IN_USE
        );
        assert_eq!(std::fs::read(&path).unwrap(), vec![0u8; 8192]);
        let _ = std::fs::remove_file(path);
    }

    fn card_file(name: &str, len: u64) -> (std::path::PathBuf, std::fs::File) {
        let path = std::env::temp_dir().join(format!("windisk-{name}-{}", std::process::id()));
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        file.set_len(len).unwrap();
        (path, file)
    }

    #[test]
    fn only_card_readers_off_the_system_disk_are_listed() {
        let probes = [
            probe(0, 17, 512_000_000_000),
            probe(1, WINDISK_BUS_USB, 31_914_983_424),
            probe(2, WINDISK_BUS_SD, 15_931_539_456),
            probe(3, WINDISK_BUS_USB, 64_000_000_000),
            probe(4, WINDISK_BUS_USB, 0),
            probe(5, WINDISK_BUS_FILE_BACKED_VIRTUAL, 6_442_450_944),
            probe(6, WINDISK_BUS_MMC, 7_948_206_080),
        ];
        let ids = |virtual_disks: bool| -> Vec<String> {
            cards(&probes, &[0, 3], virtual_disks)
                .into_iter()
                .map(|d| d.id)
                .collect()
        };
        assert_eq!(
            ids(false),
            [
                r"\\.\PhysicalDrive1",
                r"\\.\PhysicalDrive2",
                r"\\.\PhysicalDrive6"
            ],
            "the NVMe disk, the disk the app runs from and a reader with no card are left out"
        );
        assert_eq!(
            ids(true),
            [
                r"\\.\PhysicalDrive1",
                r"\\.\PhysicalDrive2",
                r"\\.\PhysicalDrive5",
                r"\\.\PhysicalDrive6"
            ]
        );
    }

    #[test]
    fn virtual_disks_are_listed_only_when_the_test_switch_is_exactly_one() {
        assert!(virtual_disks_allowed(Some("1")));
        assert!(!virtual_disks_allowed(None));
        assert!(!virtual_disks_allowed(Some("true")));
        assert!(!virtual_disks_allowed(Some("0")));
        assert!(!writable_bus(WINDISK_BUS_FILE_BACKED_VIRTUAL, false));
    }

    #[test]
    fn a_reader_is_named_by_vendor_product_and_size() {
        let listed = cards(&[probe(1, WINDISK_BUS_USB, 31_914_983_424)], &[], false);
        assert_eq!(listed[0].name, "Generic STORAGE DEVICE · 31.9 GB");
        assert_eq!(
            label(&Probe {
                vendor: " ".into(),
                product: "".into(),
                ..probe(1, WINDISK_BUS_USB, 1)
            }),
            "Card reader"
        );
    }

    #[test]
    fn disk_ids_round_trip_and_reject_anything_else() {
        assert_eq!(disk_number(&disk_path(12)), Some(12));
        assert_eq!(disk_number(r"\\.\C:"), None);
        assert_eq!(disk_number("/dev/bus/usb/001/004"), None);
    }

    #[test]
    fn missing_rights_read_as_an_administrator_refusal() {
        let said = |code: i32| refusal(&io::Error::from_raw_os_error(code));
        assert_eq!(said(5), WINDISK_NEEDS_ADMIN);
        assert_eq!(said(32), WINDISK_IN_USE);
        assert_eq!(said(19), WINDISK_WRITE_PROTECTED);
        assert_eq!(said(2), WINDISK_NOT_CONNECTED);
        assert!(said(1117).starts_with("The card could not be opened: "));
    }

    #[test]
    fn descriptor_strings_stop_at_their_terminator() {
        let bytes = b"\0\0\0\0Generic \0STORAGE DEVICE  \0";
        assert_eq!(ascii_at(bytes, 4), "Generic");
        assert_eq!(ascii_at(bytes, 13), "STORAGE DEVICE");
        assert_eq!(ascii_at(bytes, 0), "", "offset 0 means the field is absent");
        assert_eq!(ascii_at(bytes, 400), "");
    }

    #[test]
    fn a_storage_descriptor_gives_the_bus_and_names() {
        let mut bytes = vec![0u8; 64];
        bytes[12..16].copy_from_slice(&40u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&48u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&7u32.to_le_bytes());
        bytes[40..47].copy_from_slice(b"Generic");
        bytes[48..62].copy_from_slice(b"STORAGE DEVICE");
        assert_eq!(
            described(4, &bytes, 31_914_983_424),
            Some(probe(4, WINDISK_BUS_USB, 31_914_983_424))
        );
        assert_eq!(
            described(4, &bytes[..20], 1),
            None,
            "a short answer is no answer"
        );
        let mut geometry = vec![0u8; 32];
        geometry[24..32].copy_from_slice(&6_442_450_944i64.to_le_bytes());
        assert_eq!(geometry_size(&geometry), 6_442_450_944);
        assert_eq!(geometry_size(&geometry[..30]), 0);
    }

    #[test]
    fn volume_extents_name_every_disk_they_span() {
        let extent = |disk: u32| {
            [
                disk.to_le_bytes().to_vec(),
                vec![0u8; 4],
                1_048_576i64.to_le_bytes().to_vec(),
                4_096i64.to_le_bytes().to_vec(),
            ]
            .concat()
        };
        let two = [
            2u32.to_le_bytes().to_vec(),
            vec![0u8; 4],
            extent(3),
            extent(7),
        ]
        .concat();
        assert_eq!(extent_disks(&two), [3, 7]);
        assert_eq!(
            extent_disks(&two[..WINDISK_EXTENTS_HEADER + WINDISK_EXTENT_LEN]),
            [3]
        );
        assert_eq!(extent_disks(&[]), Vec::<u32>::new());
    }

    #[test]
    fn the_end_of_a_raw_disk_is_its_capacity_not_the_handle_size() {
        let (path, file) = card_file("end", 0);
        let mut disk = RawDisk::new(file, 8192, unclaimed());
        assert_eq!(disk.seek(SeekFrom::End(-4096)).unwrap(), 4096);
        assert_eq!(
            disk.read(&mut [0u8; 8192]).unwrap(),
            0,
            "a file this short ends early"
        );
        assert_eq!(disk.seek(SeekFrom::End(0)).unwrap(), 8192);
        assert_eq!(disk.read(&mut [0u8; 512]).unwrap(), 0);
        assert_eq!(
            disk.write(&[1u8; 512]).unwrap_err().kind(),
            io::ErrorKind::WriteZero
        );
        assert_eq!(
            disk.seek(SeekFrom::Current(-9000)).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_whole_image_flashes_through_a_raw_disk() {
        let image = build_image();
        let len = (image.len() + 2 * cardwrite::BLOCK) as u64;
        let (path, file) = card_file("flash", len);
        let mut device = AlignedDevice::new(RawDisk::new(file, len, unclaimed()));
        let mut stages = Vec::new();
        cardwrite::flash(
            &mut device,
            &mut io::Cursor::new(image.clone()),
            &Params {
                image_len: image.len() as u64,
                verify: true,
            },
            &cloud_init_config(),
            &mut |stage, _, _| {
                if stages.last() != Some(&stage) {
                    stages.push(stage);
                }
            },
        )
        .expect("flash");
        assert_eq!(stages, [Stage::Write, Stage::Verify, Stage::Customize]);
        let written = std::fs::read(&path).unwrap();
        assert_eq!(&written[..512], &image[..512]);
        assert!(
            read_fat_file(&written, "user-data")
                .unwrap()
                .contains("hostname: aircast")
        );
        let _ = std::fs::remove_file(path);
    }
}
