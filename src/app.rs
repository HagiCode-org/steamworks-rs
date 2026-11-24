use super::*;

/// An id for a steam app/game
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AppId(pub u32);
impl From<u32> for AppId {
    fn from(id: u32) -> Self {
        AppId(id)
    }
}

/// Access to the steam apps interface
pub struct Apps {
    pub(crate) apps: *mut sys::ISteamApps,
    pub(crate) _inner: Arc<Inner>,
}

impl Apps {
    /// Returns whether the user currently has the app with the given
    /// ID currently installed.
    ///
    /// This does not mean the user owns the game.
    pub fn is_app_installed(&self, app_id: AppId) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsAppInstalled(self.apps, app_id.0) }
    }

    /// Returns whether the user owns the specific dlc and has it
    /// installed.
    pub fn is_dlc_installed(&self, app_id: AppId) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsDlcInstalled(self.apps, app_id.0) }
    }

    /// Returns whether the user is subscribed to the app with the given
    /// ID.
    ///
    /// This should only be used to check ownership of a game related to
    /// yours (e.g. demo).
    pub fn is_subscribed_app(&self, app_id: AppId) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsSubscribedApp(self.apps, app_id.0) }
    }

    /// Returns whether the user is subscribed via a free weekend
    pub fn is_subscribed_from_free_weekend(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsSubscribedFromFreeWeekend(self.apps) }
    }

    /// Returns whether the user has a VAC ban on their account.
    pub fn is_vac_banned(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsVACBanned(self.apps) }
    }

    /// Returns whether the license for the current app ID
    /// is for cyber cafes.
    pub fn is_cybercafe(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsCybercafe(self.apps) }
    }

    /// Returns whether the license for the current app ID
    /// provides low violence depots.
    pub fn is_low_violence(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsLowViolence(self.apps) }
    }

    /// Returns whether the user is subscribed to the current app ID
    pub fn is_subscribed(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsSubscribed(self.apps) }
    }

    /// Returns the build id of this app.
    pub fn app_build_id(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamApps_GetAppBuildId(self.apps) as i32 }
    }

    /// Returns the installation folder of the app with the given ID.
    ///
    /// This works even if the app isn't installed, returning where it
    /// would be installed in the default location.
    pub fn app_install_dir(&self, app_id: AppId) -> String {
        unsafe {
            let mut buffer = vec![0; 2048];
            sys::SteamAPI_ISteamApps_GetAppInstallDir(
                self.apps,
                app_id.0,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
            );
            let path = CStr::from_ptr(buffer.as_ptr());
            path.to_string_lossy().into_owned()
        }
    }

    /// Returns the steam id of the original owner of the app.
    ///
    /// Differs from the current user if the app is borrowed.
    pub fn app_owner(&self) -> SteamId {
        unsafe { SteamId(sys::SteamAPI_ISteamApps_GetAppOwner(self.apps)) }
    }

    /// Returns a list of languages that the current app supports.
    pub fn available_game_languages(&self) -> Vec<String> {
        unsafe {
            let langs = sys::SteamAPI_ISteamApps_GetAvailableGameLanguages(self.apps);
            let langs = CStr::from_ptr(langs);
            let langs = langs.to_string_lossy();
            langs.split(',').map(|v| v.to_owned()).collect()
        }
    }

    /// Returns the language the user has set for the current game.
    ///
    /// If the language hasn't been set this returns the language
    /// used for the steam UI.
    pub fn current_game_language(&self) -> String {
        unsafe {
            let lang = sys::SteamAPI_ISteamApps_GetCurrentGameLanguage(self.apps);
            let lang = CStr::from_ptr(lang);
            lang.to_string_lossy().into_owned()
        }
    }

    /// Returns the current beta name if any.
    ///
    /// If the user isn't playing on a beta branch then this
    /// returns `None`
    pub fn current_beta_name(&self) -> Option<String> {
        unsafe {
            let mut buffer = vec![0; 256];
            if sys::SteamAPI_ISteamApps_GetCurrentBetaName(
                self.apps,
                buffer.as_mut_ptr(),
                buffer.len() as _,
            ) {
                let path = CStr::from_ptr(buffer.as_ptr());
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            }
        }
    }

    /// Returns the command line if the game was launched via Steam URL
    ///
    /// If the game was not launched through Steam URL, this returns an empty string.
    ///
    /// See [Steam API](https://partner.steamgames.com/doc/api/ISteamApps#GetLaunchCommandLine)
    pub fn launch_command_line(&self) -> String {
        unsafe {
            let mut buffer = vec![0; 256];
            let _bytes = sys::SteamAPI_ISteamApps_GetLaunchCommandLine(
                self.apps,
                buffer.as_mut_ptr(),
                buffer.len() as _,
            );
            let command_line = CStr::from_ptr(buffer.as_ptr());
            command_line.to_string_lossy().into_owned()
        }
    }

    /// Gets the associated launch parameter if the game is run via steam://run/<appid>/?param1=value1;param2=value2;param3=value3 etc.
    ///
    /// Parameter names starting with the character '@' are reserved for internal use and will always return an empty string.
    /// Parameter names starting with an underscore '_' are reserved for steam features -- they can be queried by the game, but it is advised that you don't use param names beginning with an underscore for your own features.
    ///
    /// See [Steam API](https://partner.steamgames.com/doc/api/ISteamApps#GetLaunchQueryParam)
    pub fn launch_query_param(&self, key: &str) -> String {
        let key = CString::new(key).unwrap();
        unsafe {
            let value = sys::SteamAPI_ISteamApps_GetLaunchQueryParam(self.apps, key.as_ptr());
            let value = CStr::from_ptr(value);
            value.to_string_lossy().into_owned()
        }
    }

    /// Returns the Unix time (seconds since 1970-01-01 UTC) of the user's earliest purchase of the app.
    ///
    /// The SDK header does not say what is returned when the user has no purchase of the app;
    /// unverified against a live Steam client.
    pub fn earliest_purchase_unix_time(&self, app_id: AppId) -> u32 {
        unsafe { sys::SteamAPI_ISteamApps_GetEarliestPurchaseUnixTime(self.apps, app_id.0) }
    }

    /// Returns the number of DLC entries known for the running app. Read each one with
    /// [`dlc_data_by_index`](Self::dlc_data_by_index).
    pub fn dlc_count(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamApps_GetDLCCount(self.apps) }
    }

    /// Returns the metadata of the DLC at `index`, which must be in `0..dlc_count()`.
    ///
    /// Returns `None` when the SDK does not return the entry. The name is read up to 1023 bytes.
    pub fn dlc_data_by_index(&self, index: i32) -> Option<DlcInfo> {
        let mut app_id: u32 = 0;
        let mut available = false;
        let mut name = [0 as std::os::raw::c_char; DLC_NAME_BUFFER_LEN];
        let found = unsafe {
            sys::SteamAPI_ISteamApps_BGetDLCDataByIndex(
                self.apps,
                index,
                &mut app_id,
                &mut available,
                name.as_mut_ptr(),
                name.len() as std::os::raw::c_int,
            )
        };
        found.then(|| DlcInfo {
            app_id: AppId(app_id),
            available,
            name: crate::utils::string_from_c_buffer(&name),
        })
    }

    /// Starts the installation of an optional DLC.
    ///
    /// This changes the local installation: Steam starts downloading the DLC. Progress is read with
    /// [`dlc_download_progress`](Self::dlc_download_progress).
    pub fn install_dlc(&self, app_id: AppId) {
        unsafe { sys::SteamAPI_ISteamApps_InstallDLC(self.apps, app_id.0) }
    }

    /// Uninstalls an optional DLC.
    ///
    /// This changes the local installation of the DLC. It does not change ownership.
    pub fn uninstall_dlc(&self, app_id: AppId) {
        unsafe { sys::SteamAPI_ISteamApps_UninstallDLC(self.apps, app_id.0) }
    }

    /// Tells Steam that the game files seem corrupt or missing.
    ///
    /// This changes the local installation: Steam acts on the report. What Steam then does is
    /// unverified against a live Steam client. Returns the SDK's `bool` as it is.
    pub fn mark_content_corrupt(&self, missing_files_only: bool) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_MarkContentCorrupt(self.apps, missing_files_only) }
    }

    /// Returns the depot ids installed for the app, in mount order.
    ///
    /// At most 256 depots are read. Returns an empty list when none are reported.
    pub fn installed_depots(&self, app_id: AppId) -> Vec<u32> {
        let mut depots = [0u32; MAX_INSTALLED_DEPOTS];
        let count = unsafe {
            sys::SteamAPI_ISteamApps_GetInstalledDepots(
                self.apps,
                app_id.0,
                depots.as_mut_ptr(),
                depots.len() as u32,
            )
        };
        depots[..(count as usize).min(depots.len())].to_vec()
    }

    /// Returns the bytes downloaded and the bytes total of an optional DLC that is being installed.
    ///
    /// Returns `None` when the SDK reports no progress for the app.
    pub fn dlc_download_progress(&self, app_id: AppId) -> Option<DlcDownloadProgress> {
        let mut downloaded: u64 = 0;
        let mut total: u64 = 0;
        let reported = unsafe {
            sys::SteamAPI_ISteamApps_GetDlcDownloadProgress(
                self.apps,
                app_id.0,
                &mut downloaded,
                &mut total,
            )
        };
        reported.then_some(DlcDownloadProgress {
            bytes_downloaded: downloaded,
            bytes_total: total,
        })
    }

    /// Asynchronously reads the size, SHA-1 hash and flags of a file of the app.
    ///
    /// The answer arrives through `run_callbacks()`. The callback receives `Err(SteamError::InvalidParameter)`
    /// when the name contains a NUL byte, `Err(SteamError::IOFailure)` when the call fails, and the
    /// Steam error when the result is not OK.
    pub fn file_details<F>(&self, file_name: &str, cb: F)
    where
        F: FnOnce(Result<FileDetails, SteamError>) + 'static + Send,
    {
        let Ok(file_name) = CString::new(file_name) else {
            cb(Err(SteamError::InvalidParameter));
            return;
        };
        unsafe {
            let api_call = sys::SteamAPI_ISteamApps_GetFileDetails(self.apps, file_name.as_ptr());
            register_call_result::<sys::FileDetailsResult_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else if v.m_eResult == sys::EResult::k_EResultOK {
                        Ok(FileDetails {
                            size: v.m_ulFileSize,
                            sha1: v.m_FileSHA,
                            flags: v.m_unFlags,
                        })
                    } else {
                        Err(v.m_eResult.into())
                    })
                },
            );
        }
    }

    /// Returns whether the user has the app through Family Sharing. If so, call
    /// [`app_owner`](Self::app_owner) for the lender's Steam ID.
    pub fn is_subscribed_from_family_sharing(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_BIsSubscribedFromFamilySharing(self.apps) }
    }

    /// Returns the time limits of a timed trial, or `None` when the app is not a timed trial.
    pub fn timed_trial(&self) -> Option<TimedTrial> {
        let mut allowed: u32 = 0;
        let mut played: u32 = 0;
        let is_trial =
            unsafe { sys::SteamAPI_ISteamApps_BIsTimedTrial(self.apps, &mut allowed, &mut played) };
        is_trial.then_some(TimedTrial {
            seconds_allowed: allowed,
            seconds_played: played,
        })
    }

    /// Tells Steam which DLC the user is playing, or `0` for none. Steam uses it to track the usage of
    /// major DLC. Returns the SDK's `bool` as it is.
    pub fn set_dlc_context(&self, app_id: AppId) -> bool {
        unsafe { sys::SteamAPI_ISteamApps_SetDlcContext(self.apps, app_id.0) }
    }

    /// Returns how many beta branches are known for the app. `total` includes the default branch.
    pub fn beta_counts(&self) -> BetaCounts {
        let mut available: i32 = 0;
        let mut private: i32 = 0;
        let total = unsafe {
            sys::SteamAPI_ISteamApps_GetNumBetas(self.apps, &mut available, &mut private)
        };
        BetaCounts {
            total,
            available,
            private,
        }
    }

    /// Returns the beta branch at `index`, which is in `0..beta_counts().total`.
    ///
    /// Returns `None` when the SDK does not return the entry. The name is read up to 255 bytes and
    /// the description up to 1023 bytes.
    pub fn beta_info(&self, index: i32) -> Option<BetaInfo> {
        let mut flags: u32 = 0;
        let mut build_id: u32 = 0;
        let mut name = [0 as std::os::raw::c_char; BETA_NAME_BUFFER_LEN];
        let mut description = [0 as std::os::raw::c_char; BETA_DESCRIPTION_BUFFER_LEN];
        let found = unsafe {
            sys::SteamAPI_ISteamApps_GetBetaInfo(
                self.apps,
                index,
                &mut flags,
                &mut build_id,
                name.as_mut_ptr(),
                name.len() as std::os::raw::c_int,
                description.as_mut_ptr(),
                description.len() as std::os::raw::c_int,
            )
        };
        found.then(|| BetaInfo {
            flags,
            build_id,
            name: crate::utils::string_from_c_buffer(&name),
            description: crate::utils::string_from_c_buffer(&description),
        })
    }

    /// Selects a beta branch of the app. Pass `"public"` to return to the default branch.
    ///
    /// This changes the local installation: Steam may need the game to restart before it updates to
    /// the new branch. Returns `Err(())` when the name contains a NUL byte or Steam refuses it.
    pub fn set_active_beta(&self, name: &str) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let accepted = unsafe { sys::SteamAPI_ISteamApps_SetActiveBeta(self.apps, name.as_ptr()) };
        if accepted {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Requests the legacy proof-of-purchase (CD) key of the app or one of its DLC.
    ///
    /// The key arrives in an [`AppProofOfPurchaseKeyResponse`] callback, registered with
    /// `register_callback` or `process_callbacks`.
    pub fn request_proof_of_purchase_key(&self, app_id: AppId) {
        unsafe { sys::SteamAPI_ISteamApps_RequestAppProofOfPurchaseKey(self.apps, app_id.0) }
    }

    /// Requests every proof-of-purchase key of the app and its DLC.
    ///
    /// One [`AppProofOfPurchaseKeyResponse`] callback arrives per key, then a final one with
    /// `app_id` set to `AppId(0)`.
    pub fn request_all_proof_of_purchase_keys(&self) {
        unsafe { sys::SteamAPI_ISteamApps_RequestAllProofOfPurchaseKeys(self.apps) }
    }
}

const DLC_NAME_BUFFER_LEN: usize = 1024;
const MAX_INSTALLED_DEPOTS: usize = 256;
const BETA_NAME_BUFFER_LEN: usize = 256;
const BETA_DESCRIPTION_BUFFER_LEN: usize = 1024;

/// Metadata of one DLC, from [`Apps::dlc_data_by_index`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DlcInfo {
    /// The DLC's app id.
    pub app_id: AppId,
    /// The `available` flag the SDK reports. The header does not define it further (unverified).
    pub available: bool,
    /// The DLC name as Steam reports it.
    pub name: String,
}

/// The bytes of an optional DLC downloaded so far, from [`Apps::dlc_download_progress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DlcDownloadProgress {
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
}

/// The size, SHA-1 hash and flags of a file, from [`Apps::file_details`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FileDetails {
    /// The original file size in bytes.
    pub size: u64,
    /// The SHA-1 hash of the original file.
    pub sha1: [u8; 20],
    /// The flags as Steam reports them. The header does not define them (unverified).
    pub flags: u32,
}

/// The time limits of a timed trial, from [`Apps::timed_trial`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TimedTrial {
    /// The playtime allowed, in seconds.
    pub seconds_allowed: u32,
    /// The playtime used so far, in seconds.
    pub seconds_played: u32,
}

/// The number of beta branches of the app, from [`Apps::beta_counts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BetaCounts {
    /// Every known branch, including the default `public` branch.
    pub total: i32,
    /// The branches the user can select.
    pub available: i32,
    /// The branches that are password protected.
    pub private: i32,
}

/// A beta branch of the app, from [`Apps::beta_info`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BetaInfo {
    /// The `EBetaBranchFlags` bit set: 1 default branch, 2 available, 4 private, 8 selected, 16 installed.
    pub flags: u32,
    /// The build id the branch currently has.
    pub build_id: u32,
    /// The branch name, for example `public`.
    pub name: String,
    /// The branch description as Steam reports it.
    pub description: String,
}

/// Answer to [`Apps::request_proof_of_purchase_key`] or [`Apps::request_all_proof_of_purchase_keys`]
/// (`AppProofOfPurchaseKeyResponse_t`).
///
/// `key` is a legacy CD key. Treat it as a secret and do not log it. The final callback of
/// `request_all_proof_of_purchase_keys` has `app_id` set to `AppId(0)` and an empty key.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AppProofOfPurchaseKeyResponse {
    /// The app the key belongs to, or `AppId(0)` for the final callback of a full request.
    pub app_id: AppId,
    /// Whether Steam returned the key.
    pub result: Result<(), SteamError>,
    /// The key, when `result` is `Ok`.
    pub key: String,
}

impl_callback!(cb: AppProofOfPurchaseKeyResponse_t => AppProofOfPurchaseKeyResponse {
    let len = (cb.m_cchKeyLength as usize).min(cb.m_rgchKey.len());
    let bytes: Vec<u8> = cb.m_rgchKey[..len].iter().map(|&c| c as u8).collect();
    Self {
        app_id: AppId(cb.m_nAppID),
        result: crate::to_steam_result(cb.m_eResult),
        key: String::from_utf8_lossy(&bytes).into_owned(),
    }
});

/// Called after the user executes a steam url with command line or query parameters such as steam://run/<appid>//?param1=value1;param2=value2;param3=value3; while the game is already running.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NewUrlLaunchParameters;

impl_callback!(_cb: NewUrlLaunchParameters_t => NewUrlLaunchParameters {
    Self
});
