use std::path::Path;

#[cfg(windows)]
pub fn file_identity(path: &Path) -> std::io::Result<String> {
    use std::{fs::File, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let file = File::open(path)?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the handle remains valid for the call and `information` points to writable memory
    // of the exact structure required by GetFileInformationByHandle.
    let success = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
    if success == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let file_index = ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64;
    Ok(format!(
        "{:08x}:{:016x}",
        information.dwVolumeSerialNumber, file_index
    ))
}

#[cfg(not(windows))]
pub fn file_identity(path: &Path) -> std::io::Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = path.metadata()?;
    Ok(format!("{:016x}:{:016x}", metadata.dev(), metadata.ino()))
}
