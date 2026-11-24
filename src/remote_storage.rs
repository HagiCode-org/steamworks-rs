#[cfg(test)]
use serial_test::serial;
use {super::*, ::core::mem::MaybeUninit};

/// Access to the steam remote storage interface
pub struct RemoteStorage {
    pub(crate) rs: *mut sys::ISteamRemoteStorage,
    pub(crate) util: *mut sys::ISteamUtils,
    pub(crate) inner: Arc<Inner>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishedFileVisibility {
    Public,
    FriendsOnly,
    Private,
    Unlisted,
}

impl From<sys::ERemoteStoragePublishedFileVisibility> for PublishedFileVisibility {
    fn from(visibility: sys::ERemoteStoragePublishedFileVisibility) -> Self {
        match visibility {
            sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityPublic => PublishedFileVisibility::Public,
            sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityFriendsOnly => PublishedFileVisibility::FriendsOnly,
            sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityPrivate => PublishedFileVisibility::Private,
            sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityUnlisted => PublishedFileVisibility::Unlisted,
            _ => unreachable!(),
        }
    }
}

impl Into<sys::ERemoteStoragePublishedFileVisibility> for PublishedFileVisibility {
    fn into(self) -> sys::ERemoteStoragePublishedFileVisibility {
        match self {
            PublishedFileVisibility::Public => sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityPublic,
            PublishedFileVisibility::FriendsOnly => sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityFriendsOnly,
            PublishedFileVisibility::Private => sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityPrivate,
            PublishedFileVisibility::Unlisted => sys::ERemoteStoragePublishedFileVisibility::k_ERemoteStoragePublishedFileVisibilityUnlisted,
        }
    }
}

impl Clone for RemoteStorage {
    fn clone(&self) -> Self {
        RemoteStorage {
            inner: self.inner.clone(),
            rs: self.rs,
            util: self.util,
        }
    }
}

impl RemoteStorage {
    /// Toggles whether the steam cloud is enabled for the application
    pub fn set_cloud_enabled_for_app(&self, enabled: bool) {
        unsafe {
            sys::SteamAPI_ISteamRemoteStorage_SetCloudEnabledForApp(self.rs, enabled);
        }
    }

    /// Returns whether the steam cloud is enabled for the application
    ///
    /// # Note
    ///
    /// This is independent from the account wide setting
    pub fn is_cloud_enabled_for_app(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_IsCloudEnabledForApp(self.rs) }
    }

    /// Returns whether the steam cloud is enabled for the account
    ///
    /// # Note
    ///
    /// This is independent from the application setting
    pub fn is_cloud_enabled_for_account(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_IsCloudEnabledForAccount(self.rs) }
    }

    /// Returns information about all files in the cloud storage
    pub fn files(&self) -> Vec<SteamFileInfo> {
        unsafe {
            let count = sys::SteamAPI_ISteamRemoteStorage_GetFileCount(self.rs);
            if count == -1 {
                return Vec::new();
            }
            let mut files = Vec::with_capacity(count as usize);
            for idx in 0..count {
                let mut size = 0;
                let name = CStr::from_ptr(sys::SteamAPI_ISteamRemoteStorage_GetFileNameAndSize(
                    self.rs, idx, &mut size,
                ));
                files.push(SteamFileInfo {
                    name: name.to_string_lossy().into_owned(),
                    size: size as u64,
                })
            }

            files
        }
    }

    /// Returns a handle to a steam cloud file
    ///
    /// The file does not have to exist.
    pub fn file(&self, name: &str) -> SteamFile {
        SteamFile {
            rs: self.rs,
            util: self.util,
            _inner: self.inner.clone(),
            name: CString::new(name).unwrap(),
        }
    }
}

bitflags! {
    /// Platform flags used with [`SteamFile::set_sync_platforms`] to restrict
    /// syncing to specific operating systems.
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[repr(C)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct RemoteStoragePlatforms: u32 {
        const WINDOWS = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformWindows.0 as _;
        const MACOS = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformOSX.0 as _;
        const PS3 = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformPS3.0 as _;
        const LINUX = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformLinux.0 as _;
        const SWITCH = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformSwitch.0 as _;
        const ANDROID = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformAndroid.0 as _;
        const IOS = sys::ERemoteStoragePlatform::k_ERemoteStoragePlatformIOS.0 as _;
    }
}

impl From<RemoteStoragePlatforms> for sys::ERemoteStoragePlatform {
    fn from(platforms: RemoteStoragePlatforms) -> Self {
        sys::ERemoteStoragePlatform(platforms.bits() as _)
    }
}

/// A handle for a possible steam cloud file
pub struct SteamFile {
    pub(crate) rs: *mut sys::ISteamRemoteStorage,
    pub(crate) util: *mut sys::ISteamUtils,
    pub(crate) _inner: Arc<Inner>,
    name: CString,
}

impl SteamFile {
    /// Deletes the file locally and remotely.
    ///
    /// Returns whether a file was actually deleted
    pub fn delete(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_FileDelete(self.rs, self.name.as_ptr()) }
    }
    /// Deletes the file remotely whilst keeping it locally.
    ///
    /// Returns whether a file was actually forgotten
    pub fn forget(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_FileForget(self.rs, self.name.as_ptr()) }
    }

    /// Returns whether a file exists
    pub fn exists(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_FileExists(self.rs, self.name.as_ptr()) }
    }

    /// Returns whether a file is persisted in the steam cloud
    pub fn is_persisted(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_FilePersisted(self.rs, self.name.as_ptr()) }
    }

    /// Returns the timestamp of the file
    pub fn timestamp(&self) -> i64 {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_GetFileTimestamp(self.rs, self.name.as_ptr()) }
    }

    /// Set which platforms the file should be available on
    pub fn set_sync_platforms(&self, platforms: RemoteStoragePlatforms) {
        unsafe {
            sys::SteamAPI_ISteamRemoteStorage_SetSyncPlatforms(
                self.rs,
                self.name.as_ptr(),
                platforms.into(),
            );
        }
    }

    /// Returns the platforms the file is available on
    pub fn get_sync_platforms(&self) -> RemoteStoragePlatforms {
        let bits = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_GetSyncPlatforms(self.rs, self.name.as_ptr())
        };
        RemoteStoragePlatforms::from_bits_truncate(bits.0 as _)
    }

    pub fn write(self) -> SteamFileWriter {
        unsafe {
            let handle =
                sys::SteamAPI_ISteamRemoteStorage_FileWriteStreamOpen(self.rs, self.name.as_ptr());
            SteamFileWriter {
                file: self,
                handle,
                finished: false,
            }
        }
    }

    pub fn read(self) -> SteamFileReader {
        unsafe {
            SteamFileReader {
                offset: 0,
                size: sys::SteamAPI_ISteamRemoteStorage_GetFileSize(self.rs, self.name.as_ptr())
                    as usize,
                file: self,
            }
        }
    }

    pub fn share(&self, cb: impl FnOnce(Result<u64, SteamError>) + 'static + Send) {
        let api_call =
            unsafe { sys::SteamAPI_ISteamRemoteStorage_FileShare(self.rs, self.name.as_ptr()) };
        unsafe {
            register_call_result::<sys::RemoteStorageFileShareResult_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    if io_error {
                        cb(Err(SteamError::IOFailure));
                        return;
                    }
                    if v.m_eResult != sys::EResult::k_EResultOK {
                        cb(Err(v.m_eResult.into()));
                        return;
                    }

                    cb(Ok(v.m_hFile))
                },
            )
        }
    }
}

/// A write handle for a steam cloud file
pub struct SteamFileWriter {
    file: SteamFile,
    handle: sys::UGCFileWriteStreamHandle_t,
    /// Set by `close` and `cancel`, so that `Drop` does not close the stream a second time.
    finished: bool,
}

impl std::io::Write for SteamFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            if sys::SteamAPI_ISteamRemoteStorage_FileWriteStreamWriteChunk(
                self.file.rs,
                self.handle,
                buf.as_ptr().cast(),
                buf.len() as _,
            ) {
                Ok(buf.len())
            } else {
                Err(std::io::ErrorKind::Other.into())
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for SteamFileWriter {
    fn drop(&mut self) {
        // A stream Steam never opened has nothing to close.
        if self.finished || self.handle == sys::k_UGCFileStreamHandleInvalid {
            return;
        }
        unsafe {
            sys::SteamAPI_ISteamRemoteStorage_FileWriteStreamClose(self.file.rs, self.handle);
        }
    }
}

/// A read handle for a steam cloud file
pub struct SteamFileReader {
    file: SteamFile,
    offset: usize,
    size: usize,
}

impl std::io::Read for SteamFileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::cmp::min;
        if buf.is_empty() || self.size - self.offset == 0 {
            return Ok(0);
        }
        let len = min(buf.len(), self.size - self.offset);
        unsafe {
            let api_call = sys::SteamAPI_ISteamRemoteStorage_FileReadAsync(
                self.file.rs,
                self.file.name.as_ptr(),
                self.offset as _,
                len as _,
            );

            let mut failed = false;
            while !sys::SteamAPI_ISteamUtils_IsAPICallCompleted(
                self.file.util,
                api_call,
                &mut failed,
            ) {
                std::thread::yield_now();
            }
            if failed {
                return Err(std::io::ErrorKind::Other.into());
            }
            let mut callback: sys::RemoteStorageFileReadAsyncComplete_t = std::mem::zeroed();
            sys::SteamAPI_ISteamUtils_GetAPICallResult(
                self.file.util,
                api_call,
                (&mut callback) as *mut _ as *mut _,
                std::mem::size_of::<sys::RemoteStorageFileReadAsyncComplete_t>() as _,
                1332,
                &mut failed,
            );

            if callback.m_eResult != sys::EResult::k_EResultOK {
                return Err(std::io::ErrorKind::Other.into());
            }
            let size = callback.m_cubRead as usize;
            sys::SteamAPI_ISteamRemoteStorage_FileReadAsyncComplete(
                self.file.rs,
                callback.m_hFileReadAsync,
                buf.as_mut_ptr().cast(),
                callback.m_cubRead,
            );

            self.offset += size;
            Ok(size)
        }
    }
}

impl std::io::Seek for SteamFileReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Current(o) => {
                if self.offset as isize + o as isize >= self.size as isize {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.offset = (self.offset as isize + o as isize) as usize;
            }
            std::io::SeekFrom::End(o) => {
                if o as isize >= self.size as isize {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.offset = (self.size as isize - 1 - o as isize) as usize;
            }
            std::io::SeekFrom::Start(o) => {
                if o as usize >= self.size {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.offset = o as usize;
            }
        }
        Ok(self.offset as u64)
    }
}

/// Name and size information about a file in the steam cloud
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamFileInfo {
    /// The file name
    pub name: String,
    /// The size of the file in bytes
    pub size: u64,
}

#[test]
#[serial]
fn test_cloud() {
    use std::io::{Read, Write};
    let client = Client::init().unwrap();

    let rs = client.remote_storage();
    println!("Listing files:");
    for f in rs.files() {
        println!("{:?}", f);
    }

    {
        let test = rs.file("test.txt");
        let mut w = test.write();
        write!(w, "Testing").unwrap();
    }

    println!("Listing files:");
    for f in rs.files() {
        println!("{:?}", f);
    }

    let mut output = String::new();
    let test = rs.file("test.txt");
    test.read().read_to_string(&mut output).unwrap();
    println!("Got: {:?}", output);

    assert_eq!(output, "Testing");
}

/// Steam Cloud quota of the current user, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CloudQuota {
    /// The total quota.
    pub total_bytes: u64,
    /// The part of the quota that is still available.
    pub available_bytes: u64,
}

/// What happened to a cloud file that changed on another device, from `ERemoteStorageLocalFileChange`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LocalFileChangeType {
    /// The SDK reports no change.
    Invalid,
    /// The file was updated from another device.
    FileUpdated,
    /// The file was deleted by another device.
    FileDeleted,
    /// A value this binding does not name. The SDK enum is `#[non_exhaustive]`.
    Unknown,
}

impl From<sys::ERemoteStorageLocalFileChange> for LocalFileChangeType {
    fn from(change: sys::ERemoteStorageLocalFileChange) -> Self {
        match change {
            sys::ERemoteStorageLocalFileChange::k_ERemoteStorageLocalFileChange_Invalid => {
                LocalFileChangeType::Invalid
            }
            sys::ERemoteStorageLocalFileChange::k_ERemoteStorageLocalFileChange_FileUpdated => {
                LocalFileChangeType::FileUpdated
            }
            sys::ERemoteStorageLocalFileChange::k_ERemoteStorageLocalFileChange_FileDeleted => {
                LocalFileChangeType::FileDeleted
            }
            _ => LocalFileChangeType::Unknown,
        }
    }
}

/// How the path of a changed file is given, from `ERemoteStorageFilePathType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FilePathType {
    /// The SDK reports no path type.
    Invalid,
    /// The game reads the file directly, and the path is the full path.
    Absolute,
    /// The file is accessed through the `ISteamRemoteStorage` API, and the path is the file name.
    ApiFilename,
    /// A value this binding does not name. The SDK enum is `#[non_exhaustive]`.
    Unknown,
}

impl From<sys::ERemoteStorageFilePathType> for FilePathType {
    fn from(path_type: sys::ERemoteStorageFilePathType) -> Self {
        match path_type {
            sys::ERemoteStorageFilePathType::k_ERemoteStorageFilePathType_Invalid => {
                FilePathType::Invalid
            }
            sys::ERemoteStorageFilePathType::k_ERemoteStorageFilePathType_Absolute => {
                FilePathType::Absolute
            }
            sys::ERemoteStorageFilePathType::k_ERemoteStorageFilePathType_APIFilename => {
                FilePathType::ApiFilename
            }
            _ => FilePathType::Unknown,
        }
    }
}

/// A file that changed locally after syncing, from `GetLocalFileChange`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LocalFileChange {
    /// The path or file name, as the SDK gives it.
    pub path: String,
    /// What happened to the file.
    pub change_type: LocalFileChangeType,
    /// How `path` is given.
    pub path_type: FilePathType,
}

/// The result of a UGC download, from `RemoteStorageDownloadUGCResult_t`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UgcDownload {
    /// The handle of the downloaded file.
    pub handle: u64,
    /// The app that created the file.
    pub app_id: AppId,
    /// The size of the downloaded file in bytes.
    pub size_in_bytes: i32,
    /// The file name the creator gave it.
    pub file_name: String,
    /// The Steam id of the user who created the content.
    pub owner: SteamId,
}

/// Download progress of a UGC file, from `GetUGCDownloadProgress`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UgcDownloadProgress {
    /// The number of bytes downloaded so far.
    pub bytes_downloaded: i32,
    /// The number of bytes expected. The header says this can be `0` before the transfer starts, so check it before dividing.
    pub bytes_expected: i32,
}

/// Metadata of a downloaded UGC file, from `GetUGCDetails`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UgcDetails {
    /// The app that created the file.
    pub app_id: AppId,
    /// The file name the creator gave it. Empty when the SDK returns no name.
    pub file_name: String,
    /// The size of the file in bytes.
    pub size_in_bytes: i32,
    /// The Steam id of the user who created the content.
    pub owner: SteamId,
}

/// How `ugc_read` treats the file handle after a read, from `EUGCReadAction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UgcReadAction {
    /// Keeps the handle open until the last byte is read, then closes it. Use this for sequential reads.
    ContinueReadingUntilFinished,
    /// Keeps the handle open, so the file can be read at several offsets. Finish with `Close`.
    ContinueReading,
    /// Frees the handle. Read the file again only after another `ugc_download`.
    Close,
}

impl From<UgcReadAction> for sys::EUGCReadAction {
    fn from(action: UgcReadAction) -> Self {
        match action {
            UgcReadAction::ContinueReadingUntilFinished => {
                sys::EUGCReadAction::k_EUGCRead_ContinueReadingUntilFinished
            }
            UgcReadAction::ContinueReading => sys::EUGCReadAction::k_EUGCRead_ContinueReading,
            UgcReadAction::Close => sys::EUGCReadAction::k_EUGCRead_Close,
        }
    }
}

fn ugc_download_result(
    v: &sys::RemoteStorageDownloadUGCResult_t,
) -> Result<UgcDownload, SteamError> {
    if v.m_eResult != sys::EResult::k_EResultOK {
        return Err(v.m_eResult.into());
    }
    Ok(UgcDownload {
        handle: v.m_hFile,
        app_id: AppId(v.m_nAppID),
        size_in_bytes: v.m_nSizeInBytes,
        file_name: crate::user::c_chars_to_string(&v.m_pchFileName),
        owner: SteamId(v.m_ulSteamIDOwner),
    })
}

fn register_ugc_download(
    inner: &Arc<Inner>,
    api_call: sys::SteamAPICall_t,
    cb: impl FnOnce(Result<UgcDownload, SteamError>) + 'static + Send,
) {
    unsafe {
        register_call_result::<sys::RemoteStorageDownloadUGCResult_t, _>(
            inner,
            api_call,
            move |v, io_error| {
                cb(if io_error {
                    Err(SteamError::IOFailure)
                } else {
                    ugc_download_result(v)
                })
            },
        );
    }
}

impl RemoteStorage {
    /// Returns the Steam Cloud quota of the current user in bytes, or `None` when the SDK reports no quota.
    ///
    /// The header does not say whether the quota is per account or per app; unverified against a live Steam client.
    pub fn quota(&self) -> Option<CloudQuota> {
        let mut total = 0u64;
        let mut available = 0u64;
        let ok = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_GetQuota(self.rs, &mut total, &mut available)
        };
        ok.then_some(CloudQuota {
            total_bytes: total,
            available_bytes: available,
        })
    }

    /// Replaces the content of a Steam Cloud file with `data`, asynchronously.
    ///
    /// `cb` runs with `Ok(())` once Steam has stored the file, or with Steam's error. The result arrives through `run_callbacks()`.
    /// The bytes are owned by the wrapper until the result arrives. This sends data to Steam.
    /// Returns `Err(())` without sending anything if `name` contains a NUL byte or `data` is longer than `u32::MAX` bytes.
    /// The header caps a single chunk at 100 MB and a file at 200 MB; Steam enforces those limits.
    pub fn write_file_async(
        &self,
        name: &str,
        data: Vec<u8>,
        cb: impl FnOnce(Result<(), SteamError>) + 'static + Send,
    ) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let len = u32::try_from(data.len()).map_err(|_| ())?;
        let api_call = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_FileWriteAsync(
                self.rs,
                name.as_ptr(),
                data.as_ptr().cast(),
                len,
            )
        };
        unsafe {
            register_call_result::<sys::RemoteStorageFileWriteAsyncComplete_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    // Keeps the bytes alive until Steam has answered.
                    let _data = &data;
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else if v.m_eResult == sys::EResult::k_EResultOK {
                        Ok(())
                    } else {
                        Err(v.m_eResult.into())
                    })
                },
            );
        }
        Ok(())
    }

    /// Starts a batch of file writes, so that Steam treats several file changes as one, for example the two files of one save.
    ///
    /// Each call must be paired with `end_file_write_batch`. Returns the SDK's result.
    pub fn begin_file_write_batch(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_BeginFileWriteBatch(self.rs) }
    }

    /// Ends a batch started with `begin_file_write_batch`. Returns the SDK's result.
    pub fn end_file_write_batch(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_EndFileWriteBatch(self.rs) }
    }

    /// Returns the cloud files that changed on another device since the last call, as `LocalFileChange` entries.
    ///
    /// The SDK queues these changes. Entries for which the SDK returns no path are skipped.
    pub fn local_file_changes(&self) -> Vec<LocalFileChange> {
        unsafe {
            let count = sys::SteamAPI_ISteamRemoteStorage_GetLocalFileChangeCount(self.rs);
            let mut changes = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
            for index in 0..count {
                let mut change_type =
                    sys::ERemoteStorageLocalFileChange::k_ERemoteStorageLocalFileChange_Invalid;
                let mut path_type =
                    sys::ERemoteStorageFilePathType::k_ERemoteStorageFilePathType_Invalid;
                let path = sys::SteamAPI_ISteamRemoteStorage_GetLocalFileChange(
                    self.rs,
                    index,
                    &mut change_type,
                    &mut path_type,
                );
                if path.is_null() {
                    continue;
                }
                changes.push(LocalFileChange {
                    path: CStr::from_ptr(path).to_string_lossy().into_owned(),
                    change_type: change_type.into(),
                    path_type: path_type.into(),
                });
            }
            changes
        }
    }

    /// Downloads a UGC file (user-generated content such as a Workshop file) identified by `handle`.
    ///
    /// `priority` 0 downloads at once. A higher value waits until downloads with lower values have finished, and equal values
    /// download at the same time (from the header). `cb` runs with the file's details once the download has completed; the
    /// result arrives through `run_callbacks()`. This starts network transfers.
    pub fn ugc_download(
        &self,
        handle: u64,
        priority: u32,
        cb: impl FnOnce(Result<UgcDownload, SteamError>) + 'static + Send,
    ) {
        let api_call =
            unsafe { sys::SteamAPI_ISteamRemoteStorage_UGCDownload(self.rs, handle, priority) };
        register_ugc_download(&self.inner, api_call, cb);
    }

    /// Downloads a UGC file like `ugc_download`, but to the path `location`.
    ///
    /// The header gives no detail on the location format; unverified against a live Steam client. This starts network transfers.
    /// Returns `Err(())` without downloading if `location` contains a NUL byte.
    pub fn ugc_download_to_location(
        &self,
        handle: u64,
        location: &str,
        priority: u32,
        cb: impl FnOnce(Result<UgcDownload, SteamError>) + 'static + Send,
    ) -> Result<(), ()> {
        let location = CString::new(location).map_err(|_| ())?;
        let api_call = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_UGCDownloadToLocation(
                self.rs,
                handle,
                location.as_ptr(),
                priority,
            )
        };
        register_ugc_download(&self.inner, api_call, cb);
        Ok(())
    }

    /// Returns how far the download of a UGC file has progressed, or `None` when the SDK reports no progress.
    pub fn ugc_download_progress(&self, handle: u64) -> Option<UgcDownloadProgress> {
        let mut downloaded = 0i32;
        let mut expected = 0i32;
        let ok = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_GetUGCDownloadProgress(
                self.rs,
                handle,
                &mut downloaded,
                &mut expected,
            )
        };
        ok.then_some(UgcDownloadProgress {
            bytes_downloaded: downloaded,
            bytes_expected: expected,
        })
    }

    /// Returns the metadata of a UGC file that has been downloaded, or `None` when the SDK reports no details.
    ///
    /// The metadata is the same as in the `UgcDownload` result of `ugc_download`.
    pub fn ugc_details(&self, handle: u64) -> Option<UgcDetails> {
        let mut app_id: sys::AppId_t = 0;
        let mut name: *mut c_char = std::ptr::null_mut();
        let mut size = 0i32;
        let mut owner: sys::CSteamID = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_GetUGCDetails(
                self.rs,
                handle,
                &mut app_id,
                &mut name,
                &mut size,
                &mut owner,
            )
        };
        if !ok {
            return None;
        }
        // The name points into memory Steam owns, so it is copied straight away.
        let file_name = if name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() }
        };
        Some(UgcDetails {
            app_id: AppId(app_id),
            file_name,
            size_in_bytes: size,
            owner: SteamId(unsafe { owner.m_steamid.m_unAll64Bits }),
        })
    }

    /// Reads a downloaded UGC file into `buffer`, starting at `offset`, and returns the number of bytes read.
    ///
    /// The read length is the length of `buffer`. Files over 100 MB must be read in chunks (from the header). `offset` is a `u32`, as in the SDK.
    /// Returns `None` when the SDK returns a negative count. The header does not name the failure value, so that case is unverified.
    /// Returns `None` also when `buffer` is longer than `i32::MAX` bytes.
    pub fn ugc_read(
        &self,
        handle: u64,
        buffer: &mut [u8],
        offset: u32,
        action: UgcReadAction,
    ) -> Option<usize> {
        let len = i32::try_from(buffer.len()).ok()?;
        let read = unsafe {
            sys::SteamAPI_ISteamRemoteStorage_UGCRead(
                self.rs,
                handle,
                buffer.as_mut_ptr().cast(),
                len,
                offset,
                action.into(),
            )
        };
        usize::try_from(read).ok()
    }

    /// Returns how many downloaded UGC files are waiting to be read with `ugc_read`.
    pub fn cached_ugc_count(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamRemoteStorage_GetCachedUGCCount(self.rs) }
    }

    /// Returns the handle of a downloaded UGC file that is waiting to be read, by its index.
    ///
    /// Returns `None` when the SDK returns `k_UGCHandleInvalid`, which the header defines as the invalid handle.
    pub fn cached_ugc_handle(&self, index: i32) -> Option<u64> {
        let handle =
            unsafe { sys::SteamAPI_ISteamRemoteStorage_GetCachedUGCHandle(self.rs, index) };
        if handle == sys::k_UGCHandleInvalid {
            None
        } else {
            Some(handle)
        }
    }
}

impl SteamFileWriter {
    /// Returns whether Steam opened the write stream. It is `false` when the handle is `k_UGCFileStreamHandleInvalid`.
    pub fn is_open(&self) -> bool {
        self.handle != sys::k_UGCFileStreamHandleInvalid
    }

    /// Writes one chunk to the open stream. Returns `false` when Steam refuses the chunk, or when the chunk is longer than `i32::MAX` bytes
    /// (the SDK's `int32` length); the header caps a single chunk at 100 MB. Nothing is committed until `close`.
    pub fn write_chunk(&mut self, data: &[u8]) -> bool {
        let Ok(len) = i32::try_from(data.len()) else {
            return false;
        };
        unsafe {
            sys::SteamAPI_ISteamRemoteStorage_FileWriteStreamWriteChunk(
                self.file.rs,
                self.handle,
                data.as_ptr().cast(),
                len,
            )
        }
    }

    /// Commits the stream, so the chunks written so far become the file's content. Returns whether Steam accepted the close.
    pub fn close(mut self) -> bool {
        self.finished = true;
        unsafe { sys::SteamAPI_ISteamRemoteStorage_FileWriteStreamClose(self.file.rs, self.handle) }
    }

    /// Cancels the stream without committing the chunks, using the SDK's `FileWriteStreamCancel`.
    /// Returns whether the SDK accepted the cancel. The header does not describe the effect on the existing file; unverified against a live Steam client.
    pub fn cancel(mut self) -> bool {
        self.finished = true;
        unsafe {
            sys::SteamAPI_ISteamRemoteStorage_FileWriteStreamCancel(self.file.rs, self.handle)
        }
    }
}
