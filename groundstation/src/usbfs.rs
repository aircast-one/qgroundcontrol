const DESCRIPTOR_INTERFACE: u8 = 4;
const DESCRIPTOR_ENDPOINT: u8 = 5;
const CLASS_MASS_STORAGE: u8 = 0x08;
const SUBCLASS_SCSI: u8 = 0x06;
const PROTOCOL_BULK_ONLY: u8 = 0x50;
const TRANSFER_BULK: u8 = 0x02;
const ENDPOINT_IN: u8 = 0x80;
const DEVICE_DESCRIPTOR_LEN: usize = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MassStorage {
    pub interface: u8,
    pub endpoint_in: u8,
    pub endpoint_out: u8,
}

#[derive(Default)]
struct Walk {
    interface: Option<(u8, bool)>,
    endpoint_in: Option<u8>,
    endpoint_out: Option<u8>,
    found: Option<MassStorage>,
}

pub fn mass_storage(descriptors: &[u8]) -> Option<MassStorage> {
    let records = std::iter::successors(Some(DEVICE_DESCRIPTOR_LEN), |&at| descriptors.get(at).map(|&len| at + usize::from(len.max(2))))
        .take_while(|&at| at + 2 <= descriptors.len())
        .map(|at| &descriptors[at..(at + usize::from(descriptors[at].max(2))).min(descriptors.len())]);
    records
        .fold(Walk::default(), |walk, record| match (walk.found, record.get(1).copied()) {
            (Some(_), _) => walk,
            (None, Some(DESCRIPTOR_INTERFACE)) if record.len() >= 8 => Walk {
                interface: Some((record[2], record[5] == CLASS_MASS_STORAGE && record[6] == SUBCLASS_SCSI && record[7] == PROTOCOL_BULK_ONLY)),
                ..Walk::default()
            },
            (None, Some(DESCRIPTOR_ENDPOINT)) if record.len() >= 4 && record[3] & 0x03 == TRANSFER_BULK => {
                let (endpoint_in, endpoint_out) = match record[2] & ENDPOINT_IN {
                    0 => (walk.endpoint_in, walk.endpoint_out.or(Some(record[2]))),
                    _ => (walk.endpoint_in.or(Some(record[2])), walk.endpoint_out),
                };
                let found = match (walk.interface, endpoint_in, endpoint_out) {
                    (Some((interface, true)), Some(endpoint_in), Some(endpoint_out)) => Some(MassStorage { interface, endpoint_in, endpoint_out }),
                    _ => None,
                };
                Walk { endpoint_in, endpoint_out, found, ..walk }
            }
            _ => walk,
        })
        .found
}

#[cfg(any(target_os = "android", target_os = "linux"))]
mod transport {
    use std::ffi::{c_int, c_void};
    use std::io;
    use std::os::fd::RawFd;

    use super::{MassStorage, mass_storage};
    use crate::usbmsc::Bulk;

    const TIMEOUT_MS: u32 = 10_000;
    const DESCRIPTORS_MAX: usize = 4096;

    #[repr(C)]
    struct BulkTransfer {
        endpoint: u32,
        len: u32,
        timeout: u32,
        data: *mut c_void,
    }

    #[repr(C)]
    struct InterfaceIoctl {
        interface: c_int,
        code: c_int,
        data: *mut c_void,
    }

    const fn usbdevfs(direction: u32, number: u32, size: usize) -> u32 {
        (direction << 30) | ((size as u32) << 16) | (0x55 << 8) | number
    }

    const BULK: u32 = usbdevfs(3, 2, std::mem::size_of::<BulkTransfer>());
    const CLAIM_INTERFACE: u32 = usbdevfs(2, 15, 4);
    const RELEASE_INTERFACE: u32 = usbdevfs(2, 16, 4);
    const INTERFACE_IOCTL: u32 = usbdevfs(3, 18, std::mem::size_of::<InterfaceIoctl>());
    const CLEAR_HALT: u32 = usbdevfs(2, 21, 4);
    const DISCONNECT: u32 = usbdevfs(0, 22, 0);

    fn checked(result: c_int) -> io::Result<c_int> {
        match result < 0 {
            true => Err(io::Error::last_os_error()),
            false => Ok(result),
        }
    }

    pub struct Transport {
        fd: RawFd,
        endpoints: MassStorage,
    }

    impl Transport {
        pub fn claim(fd: RawFd) -> io::Result<Self> {
            let mut descriptors = vec![0u8; DESCRIPTORS_MAX];
            let read = checked(unsafe { libc::pread(fd, descriptors.as_mut_ptr().cast(), descriptors.len(), 0) } as c_int)? as usize;
            let endpoints = mass_storage(&descriptors[..read]).ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "this USB device is not a card reader"))?;
            let mut interface = c_int::from(endpoints.interface);
            let mut detach = InterfaceIoctl { interface, code: DISCONNECT as c_int, data: std::ptr::null_mut() };
            let _ = unsafe { libc::ioctl(fd, INTERFACE_IOCTL as _, &mut detach) };
            checked(unsafe { libc::ioctl(fd, CLAIM_INTERFACE as _, &mut interface) })?;
            Ok(Self { fd, endpoints })
        }

        fn transfer(&mut self, endpoint: u8, data: *mut u8, len: usize) -> io::Result<usize> {
            let mut request = BulkTransfer { endpoint: u32::from(endpoint), len: len as u32, timeout: TIMEOUT_MS, data: data.cast() };
            checked(unsafe { libc::ioctl(self.fd, BULK as _, &mut request) }).map(|n| n as usize)
        }
    }

    impl Bulk for Transport {
        fn send(&mut self, data: &[u8]) -> io::Result<()> {
            let mut sent = 0;
            while sent < data.len() {
                let rest = &data[sent..];
                match self.transfer(self.endpoints.endpoint_out, rest.as_ptr() as *mut u8, rest.len())? {
                    0 => return Err(io::Error::new(io::ErrorKind::WriteZero, "the card reader stopped accepting data")),
                    n => sent += n,
                }
            }
            Ok(())
        }

        fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.transfer(self.endpoints.endpoint_in, buf.as_mut_ptr(), buf.len())
        }

        fn clear_halt(&mut self, inbound: bool) -> io::Result<()> {
            let mut endpoint = c_int::from(match inbound {
                true => self.endpoints.endpoint_in,
                false => self.endpoints.endpoint_out,
            });
            checked(unsafe { libc::ioctl(self.fd, CLEAR_HALT as _, &mut endpoint) }).map(|_| ())
        }
    }

    impl Drop for Transport {
        fn drop(&mut self) {
            let mut interface = c_int::from(self.endpoints.interface);
            let _ = unsafe { libc::ioctl(self.fd, RELEASE_INTERFACE as _, &mut interface) };
        }
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
pub use transport::Transport;

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> Vec<u8> {
        vec![18, 1, 0x00, 0x02, 0, 0, 0, 64, 0x05, 0x8f, 0x38, 0x63, 0x00, 0x01, 1, 2, 3, 1]
    }

    fn config(total: u16, interfaces: u8) -> Vec<u8> {
        let t = total.to_le_bytes();
        vec![9, 2, t[0], t[1], interfaces, 1, 0, 0x80, 50]
    }

    fn interface(number: u8, class: u8, subclass: u8, protocol: u8, endpoints: u8) -> Vec<u8> {
        vec![9, 4, number, 0, endpoints, class, subclass, protocol, 0]
    }

    fn endpoint(address: u8, attributes: u8) -> Vec<u8> {
        vec![7, 5, address, attributes, 0x00, 0x02, 0]
    }

    #[test]
    fn a_card_reader_exposes_its_bulk_only_interface_and_endpoints() {
        let bytes = [device(), config(32, 1), interface(0, 0x08, 0x06, 0x50, 2), endpoint(0x81, 0x02), endpoint(0x02, 0x02)].concat();
        assert_eq!(mass_storage(&bytes), Some(MassStorage { interface: 0, endpoint_in: 0x81, endpoint_out: 0x02 }));
    }

    #[test]
    fn a_composite_device_is_searched_past_its_other_interfaces() {
        let bytes = [device(), config(55, 2), interface(0, 0x03, 0x01, 0x01, 1), endpoint(0x83, 0x03), interface(1, 0x08, 0x06, 0x50, 2), endpoint(0x02, 0x02), endpoint(0x81, 0x02)].concat();
        assert_eq!(mass_storage(&bytes), Some(MassStorage { interface: 1, endpoint_in: 0x81, endpoint_out: 0x02 }));
    }

    #[test]
    fn bulk_endpoints_of_another_class_are_not_mistaken_for_storage() {
        let bytes = [device(), config(32, 1), interface(0, 0xff, 0x00, 0x00, 2), endpoint(0x81, 0x02), endpoint(0x01, 0x02)].concat();
        assert_eq!(mass_storage(&bytes), None);
    }

    #[test]
    fn a_uas_only_interface_is_not_bulk_only_transport() {
        let bytes = [device(), config(32, 1), interface(0, 0x08, 0x06, 0x62, 2), endpoint(0x81, 0x02), endpoint(0x02, 0x02)].concat();
        assert_eq!(mass_storage(&bytes), None);
    }

    #[test]
    fn truncated_descriptors_do_not_panic() {
        let bytes = [device(), config(32, 1), interface(0, 0x08, 0x06, 0x50, 2), vec![7, 5, 0x81]].concat();
        assert_eq!(mass_storage(&bytes), None);
        assert_eq!(mass_storage(&[]), None);
        assert_eq!(mass_storage(&[0u8; 40]), None);
    }
}
