use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

pub(crate) struct SingleInstanceGuard(HANDLE);

impl SingleInstanceGuard {
    pub(crate) fn acquire() -> Result<Option<Self>, String> {
        Self::named("Local\\NinjutsoBatteryTray.Singleton.v1")
    }

    fn named(name: &str) -> Result<Option<Self>, String> {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
            .map_err(|error| error.to_string())?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            let _ = unsafe { CloseHandle(handle) };
            Ok(None)
        } else {
            Ok(Some(Self(handle)))
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_mutex_prevents_two_frontends_and_is_released() {
        let name = format!(
            "Local\\NinjutsoBatteryTray.GPUI.Test.{}",
            std::process::id()
        );
        let guard = SingleInstanceGuard::named(&name).unwrap().unwrap();
        assert!(SingleInstanceGuard::named(&name).unwrap().is_none());
        drop(guard);
        assert!(SingleInstanceGuard::named(&name).unwrap().is_some());
    }
}
