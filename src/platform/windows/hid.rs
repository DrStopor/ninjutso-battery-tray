use crate::model::{ConnectionKind, MouseBattery, ReadError, ReadErrorKind};
use crate::protocol::ReadRequest;
use crate::telemetry::query_battery_with;
use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::Duration;

type Bool = i32;
type Dword = u32;
type Handle = isize;

const INVALID_HANDLE_VALUE: Handle = -1isize;
const ERROR_NO_MORE_ITEMS: u32 = 259;
const DIGCF_PRESENT: u32 = 0x0000_0002;
const DIGCF_DEVICEINTERFACE: u32 = 0x0000_0010;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;

const DEVICE_TARGETS: [DeviceTarget; 2] = [
    DeviceTarget {
        path_filter: "vid_093a&pid_e020",
        connection: ConnectionKind::Wired,
    },
    DeviceTarget {
        path_filter: "vid_093a&pid_eb01",
        connection: ConnectionKind::Receiver,
    },
];

#[derive(Clone, Copy)]
struct DeviceTarget {
    path_filter: &'static str,
    connection: ConnectionKind,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct SpDeviceInterfaceData {
    cb_size: u32,
    interface_class_guid: Guid,
    flags: u32,
    reserved: usize,
}

struct OwnedHandle(Handle);

impl OwnedHandle {
    fn new(handle: Handle) -> Result<Self, String> {
        if handle == INVALID_HANDLE_VALUE || handle == 0 {
            Err(format!(
                "CreateFileW failed: {}",
                io::Error::last_os_error()
            ))
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn read_mouse_battery() -> Result<MouseBattery, ReadError> {
    let paths = enumerate_hid_paths()?;
    let mut found_target = false;
    let mut offline_reading = None;
    let mut last_error = None;

    for target in DEVICE_TARGETS {
        for path in paths.iter().filter(|path| path_matches(path, target)) {
            found_target = true;
            match query_battery_path(path, target.connection) {
                Ok(reading) if reading.online => return Ok(reading),
                Ok(reading) => {
                    offline_reading.get_or_insert(reading);
                }
                Err(error) => {
                    last_error = Some(error);
                }
            };
        }
    }

    if let Some(reading) = offline_reading {
        return Ok(reading);
    }
    if !found_target {
        return Err(ReadError::new(
            ReadErrorKind::DeviceNotFound,
            "Ninjutso Ten was not found (expected PID_EB01 or PID_E020)",
        ));
    }

    Err(ReadError::new(
        ReadErrorKind::NoBatteryData,
        last_error.unwrap_or_else(|| "no HID collection returned battery data".to_string()),
    ))
}

fn path_matches(path: &str, target: DeviceTarget) -> bool {
    path.to_ascii_lowercase().contains(target.path_filter)
}

fn query_battery_path(path: &str, connection: ConnectionKind) -> Result<MouseBattery, String> {
    query_battery_with(connection, |request| query_feature(path, request))
}

fn query_feature(path: &str, request: ReadRequest) -> Result<[u8; 16], String> {
    let command = request.command();
    let wide_path = wide_null(path);
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    let handle = OwnedHandle::new(handle)?;

    let mut out_report = request.encode();

    let ok = unsafe { HidD_SetFeature(handle.0, out_report.as_mut_ptr(), out_report.len() as u32) };
    if ok == 0 {
        return Err(format!(
            "HidD_SetFeature command {command} failed: {}",
            io::Error::last_os_error()
        ));
    }

    thread::sleep(Duration::from_millis(30));

    let mut in_report = [0u8; 16];
    in_report[0] = 6;
    let ok = unsafe { HidD_GetFeature(handle.0, in_report.as_mut_ptr(), in_report.len() as u32) };
    if ok == 0 {
        return Err(format!(
            "HidD_GetFeature command {command} failed: {}",
            io::Error::last_os_error()
        ));
    }

    if request.accepts(&in_report) {
        Ok(in_report)
    } else {
        Err(format!(
            "unexpected HID report response for command {command}"
        ))
    }
}

fn enumerate_hid_paths() -> Result<Vec<String>, ReadError> {
    let mut hid_guid = Guid::default();
    unsafe {
        HidD_GetHidGuid(&mut hid_guid);
    }

    let info = unsafe {
        SetupDiGetClassDevsW(&hid_guid, null(), 0, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE)
    };
    if info == INVALID_HANDLE_VALUE {
        return Err(ReadError::new(
            ReadErrorKind::System,
            format!(
                "SetupDiGetClassDevsW failed: {}",
                io::Error::last_os_error()
            ),
        ));
    }

    let mut paths = Vec::new();
    let mut index = 0;
    loop {
        let mut interface = SpDeviceInterfaceData {
            cb_size: size_of::<SpDeviceInterfaceData>() as u32,
            interface_class_guid: Guid::default(),
            flags: 0,
            reserved: 0,
        };

        let ok = unsafe {
            SetupDiEnumDeviceInterfaces(info, null_mut(), &hid_guid, index, &mut interface)
        };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_NO_MORE_ITEMS {
                break;
            }
            unsafe {
                SetupDiDestroyDeviceInfoList(info);
            }
            return Err(ReadError::new(
                ReadErrorKind::System,
                format!(
                    "SetupDiEnumDeviceInterfaces failed: {}",
                    io::Error::from_raw_os_error(error as i32)
                ),
            ));
        }

        let mut required = 0u32;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                info,
                &mut interface,
                null_mut(),
                0,
                &mut required,
                null_mut(),
            );
        }

        if required > 0 {
            let mut detail = vec![0u8; required as usize];
            let cb_size = if size_of::<usize>() == 8 { 8u32 } else { 6u32 };
            unsafe {
                (detail.as_mut_ptr() as *mut u32).write_unaligned(cb_size);
            }

            let ok = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    info,
                    &mut interface,
                    detail.as_mut_ptr() as *mut c_void,
                    required,
                    &mut required,
                    null_mut(),
                )
            };
            if ok != 0 {
                let path = unsafe { wide_ptr_to_string(detail.as_ptr().add(4) as *const u16) };
                paths.push(path);
            }
        }

        index += 1;
    }

    unsafe {
        SetupDiDestroyDeviceInfoList(info);
    }
    Ok(paths)
}

unsafe fn wide_ptr_to_string(pointer: *const u16) -> String {
    let mut length = 0usize;
    while unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(pointer, length) };
    String::from_utf16_lossy(slice)
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(hid_guid: *mut Guid);
    fn HidD_SetFeature(device: Handle, report_buffer: *mut u8, report_buffer_length: u32) -> Bool;
    fn HidD_GetFeature(device: Handle, report_buffer: *mut u8, report_buffer_length: u32) -> Bool;
}

#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        hwnd_parent: Handle,
        flags: Dword,
    ) -> Handle;
    fn SetupDiEnumDeviceInterfaces(
        device_info_set: Handle,
        device_info_data: *mut c_void,
        interface_class_guid: *const Guid,
        member_index: Dword,
        device_interface_data: *mut SpDeviceInterfaceData,
    ) -> Bool;
    fn SetupDiGetDeviceInterfaceDetailW(
        device_info_set: Handle,
        device_interface_data: *mut SpDeviceInterfaceData,
        device_interface_detail_data: *mut c_void,
        device_interface_detail_data_size: Dword,
        required_size: *mut Dword,
        device_info_data: *mut c_void,
    ) -> Bool;
    fn SetupDiDestroyDeviceInfoList(device_info_set: Handle) -> Bool;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> Dword;
    fn CreateFileW(
        file_name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *mut c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: Handle,
    ) -> Handle;
    fn CloseHandle(object: Handle) -> Bool;
}

#[cfg(test)]
mod tests {
    use super::{ConnectionKind, DEVICE_TARGETS, path_matches};

    #[test]
    fn recognizes_both_ten_connection_paths() {
        let wired = r"\\?\hid#vid_093a&pid_e020&mi_02&col02#test";
        let receiver = r"\\?\hid#vid_093a&pid_eb01&mi_02&col06#test";

        assert!(path_matches(wired, DEVICE_TARGETS[0]));
        assert_eq!(DEVICE_TARGETS[0].connection, ConnectionKind::Wired);
        assert!(path_matches(receiver, DEVICE_TARGETS[1]));
        assert_eq!(DEVICE_TARGETS[1].connection, ConnectionKind::Receiver);
    }
}
