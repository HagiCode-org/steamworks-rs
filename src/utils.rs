use super::*;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic;
use std::process::abort;
use std::sync::RwLock;

/// Access to the steam utils interface
pub struct Utils {
    pub(crate) utils: *mut sys::ISteamUtils,
    pub(crate) _inner: Arc<Inner>,
}

#[derive(Clone, Debug)]
pub struct GamepadTextInputDismissed {
    pub submitted_text_len: Option<u32>,
}

impl_callback!(cb: GamepadTextInputDismissed_t => GamepadTextInputDismissed {
    Self {
        submitted_text_len: cb.m_bSubmitted.then_some(cb.m_unSubmittedText),
    }
});

#[derive(Clone, Debug)]
pub struct FloatingGamepadTextInputDismissed;

impl_callback!(_cb: FloatingGamepadTextInputDismissed_t => FloatingGamepadTextInputDismissed {
    Self
});

pub enum NotificationPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub enum GamepadTextInputMode {
    Normal,
    Password,
}

impl From<GamepadTextInputMode> for sys::EGamepadTextInputMode {
    fn from(mode: GamepadTextInputMode) -> Self {
        match mode {
            GamepadTextInputMode::Normal => {
                sys::EGamepadTextInputMode::k_EGamepadTextInputModeNormal
            }
            GamepadTextInputMode::Password => {
                sys::EGamepadTextInputMode::k_EGamepadTextInputModePassword
            }
        }
    }
}

pub enum GamepadTextInputLineMode {
    SingleLine,
    MultipleLines,
}

impl From<GamepadTextInputLineMode> for sys::EGamepadTextInputLineMode {
    fn from(mode: GamepadTextInputLineMode) -> Self {
        match mode {
            GamepadTextInputLineMode::SingleLine => {
                sys::EGamepadTextInputLineMode::k_EGamepadTextInputLineModeSingleLine
            }
            GamepadTextInputLineMode::MultipleLines => {
                sys::EGamepadTextInputLineMode::k_EGamepadTextInputLineModeMultipleLines
            }
        }
    }
}

pub enum FloatingGamepadTextInputMode {
    SingleLine,
    MultipleLines,
    Email,
    Numeric,
}

impl From<FloatingGamepadTextInputMode> for sys::EFloatingGamepadTextInputMode {
    fn from(mode: FloatingGamepadTextInputMode) -> Self {
        match mode {
            FloatingGamepadTextInputMode::SingleLine => {
                sys::EFloatingGamepadTextInputMode::k_EFloatingGamepadTextInputModeModeSingleLine
            }
            FloatingGamepadTextInputMode::MultipleLines => {
                sys::EFloatingGamepadTextInputMode::k_EFloatingGamepadTextInputModeModeMultipleLines
            }
            FloatingGamepadTextInputMode::Email => {
                sys::EFloatingGamepadTextInputMode::k_EFloatingGamepadTextInputModeModeEmail
            }
            FloatingGamepadTextInputMode::Numeric => {
                sys::EFloatingGamepadTextInputMode::k_EFloatingGamepadTextInputModeModeNumeric
            }
        }
    }
}

/// The Steam universe a client connects to (`EUniverse`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Universe {
    Invalid,
    Public,
    Beta,
    Internal,
    Dev,
}

impl Universe {
    fn from_sys(value: sys::EUniverse) -> Option<Self> {
        match value {
            sys::EUniverse::k_EUniverseInvalid => Some(Universe::Invalid),
            sys::EUniverse::k_EUniversePublic => Some(Universe::Public),
            sys::EUniverse::k_EUniverseBeta => Some(Universe::Beta),
            sys::EUniverse::k_EUniverseInternal => Some(Universe::Internal),
            sys::EUniverse::k_EUniverseDev => Some(Universe::Dev),
            _ => None,
        }
    }
}

/// The verdict of [`Utils::check_file_signature`] (`ECheckFileSignature`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileSignatureCheck {
    ValidSignature,
    InvalidSignature,
    FileNotFound,
    NoSignaturesFoundForThisApp,
    NoSignaturesFoundForThisFile,
}

impl FileSignatureCheck {
    fn from_sys(value: sys::ECheckFileSignature) -> Option<Self> {
        match value {
            sys::ECheckFileSignature::k_ECheckFileSignatureValidSignature => {
                Some(FileSignatureCheck::ValidSignature)
            }
            sys::ECheckFileSignature::k_ECheckFileSignatureInvalidSignature => {
                Some(FileSignatureCheck::InvalidSignature)
            }
            sys::ECheckFileSignature::k_ECheckFileSignatureFileNotFound => {
                Some(FileSignatureCheck::FileNotFound)
            }
            sys::ECheckFileSignature::k_ECheckFileSignatureNoSignaturesFoundForThisApp => {
                Some(FileSignatureCheck::NoSignaturesFoundForThisApp)
            }
            sys::ECheckFileSignature::k_ECheckFileSignatureNoSignaturesFoundForThisFile => {
                Some(FileSignatureCheck::NoSignaturesFoundForThisFile)
            }
            _ => None,
        }
    }
}

/// The protocol [`Utils::ipv6_connectivity_state`] tests (`ESteamIPv6ConnectivityProtocol`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ipv6Protocol {
    /// Tests through HTTP, which can differ from the other protocols when a proxy is in use.
    Http,
    /// Tests UDP on a port Steam also uses. If UDP works, TCP probably works too.
    Udp,
}

impl From<Ipv6Protocol> for sys::ESteamIPv6ConnectivityProtocol {
    fn from(protocol: Ipv6Protocol) -> Self {
        match protocol {
            Ipv6Protocol::Http => {
                sys::ESteamIPv6ConnectivityProtocol::k_ESteamIPv6ConnectivityProtocol_HTTP
            }
            Ipv6Protocol::Udp => {
                sys::ESteamIPv6ConnectivityProtocol::k_ESteamIPv6ConnectivityProtocol_UDP
            }
        }
    }
}

/// The IPv6 connectivity of this machine on one protocol (`ESteamIPv6ConnectivityState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ipv6ConnectivityState {
    /// No test has run yet.
    Unknown,
    /// A recent request over IPv6 succeeded.
    Good,
    /// A request failed, because there is no IPv6 address or no upstream connectivity.
    Bad,
}

impl Ipv6ConnectivityState {
    fn from_sys(value: sys::ESteamIPv6ConnectivityState) -> Option<Self> {
        match value {
            sys::ESteamIPv6ConnectivityState::k_ESteamIPv6ConnectivityState_Unknown => {
                Some(Ipv6ConnectivityState::Unknown)
            }
            sys::ESteamIPv6ConnectivityState::k_ESteamIPv6ConnectivityState_Good => {
                Some(Ipv6ConnectivityState::Good)
            }
            sys::ESteamIPv6ConnectivityState::k_ESteamIPv6ConnectivityState_Bad => {
                Some(Ipv6ConnectivityState::Bad)
            }
            _ => None,
        }
    }
}

/// The kind of text given to [`Utils::filter_text`] (`ETextFilteringContext`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFilteringContext {
    Unknown,
    /// Game content: only the legally required filtering is applied.
    GameContent,
    /// Chat from another player.
    Chat,
    /// A character or item name.
    Name,
}

impl From<TextFilteringContext> for sys::ETextFilteringContext {
    fn from(context: TextFilteringContext) -> Self {
        match context {
            TextFilteringContext::Unknown => {
                sys::ETextFilteringContext::k_ETextFilteringContextUnknown
            }
            TextFilteringContext::GameContent => {
                sys::ETextFilteringContext::k_ETextFilteringContextGameContent
            }
            TextFilteringContext::Chat => sys::ETextFilteringContext::k_ETextFilteringContextChat,
            TextFilteringContext::Name => sys::ETextFilteringContext::k_ETextFilteringContextName,
        }
    }
}

/// The output of [`Utils::filter_text`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilteredText {
    /// The filtered message, as UTF-8.
    pub text: String,
    /// The number of characters the filter changed, as the SDK reports it.
    pub filtered_count: i32,
}

/// Reads a NUL-terminated string from a fixed-size C buffer. A buffer without a NUL is read to its end.
pub(crate) fn string_from_c_buffer(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Global rust warning callback
static WARNING_CALLBACK: RwLock<Option<Box<dyn Fn(i32, &CStr) + Send + Sync>>> = RwLock::new(None);

/// C function to pass as the real callback, which forwards to the `WARNING_CALLBACK` if any
unsafe extern "C" fn c_warning_callback(level: i32, msg: *const c_char) {
    let lock = WARNING_CALLBACK.read().expect("warning func lock poisoned");
    let cb = match lock.as_ref() {
        Some(cb) => cb,
        None => {
            return;
        }
    };

    let s = CStr::from_ptr(msg);

    let res = panic::catch_unwind(panic::AssertUnwindSafe(|| cb(level, s)));
    if let Err(err) = res {
        if let Some(err) = err.downcast_ref::<&str>() {
            println!("Steam warning callback panicked: {}", err);
        } else if let Some(err) = err.downcast_ref::<String>() {
            println!("Steam warning callback panicked: {}", err);
        } else {
            println!("Steam warning callback panicked");
        }
        abort();
    }
}

impl Utils {
    /// Returns the app ID of the current process
    pub fn app_id(&self) -> AppId {
        unsafe { AppId(sys::SteamAPI_ISteamUtils_GetAppID(self.utils)) }
    }

    /// Returns the country code of the current user based on their IP
    pub fn ip_country(&self) -> String {
        unsafe {
            let ipcountry = sys::SteamAPI_ISteamUtils_GetIPCountry(self.utils);
            let ipcountry = CStr::from_ptr(ipcountry);
            ipcountry.to_string_lossy().into_owned()
        }
    }

    /// Returns whether or not the overlay is enabled in Steam
    pub fn is_overlay_enabled(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_IsOverlayEnabled(self.utils) }
    }

    /// Returns the language the steam client is currently
    /// running in.
    ///
    /// Generally you want `Apps::current_game_language` instead of this
    pub fn ui_language(&self) -> String {
        unsafe {
            let lang = sys::SteamAPI_ISteamUtils_GetSteamUILanguage(self.utils);
            let lang = CStr::from_ptr(lang);
            lang.to_string_lossy().into_owned()
        }
    }

    /// Returns the current real time on the Steam servers
    /// in Unix epoch format (seconds since 1970/1/1 UTC).
    pub fn get_server_real_time(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUtils_GetServerRealTime(self.utils) }
    }

    /// Sets the position on the screen where popups from the steam overlay
    /// should appear and display themselves in.
    pub fn set_overlay_notification_position(&self, position: NotificationPosition) {
        unsafe {
            let position = match position {
                NotificationPosition::TopLeft => sys::ENotificationPosition::k_EPositionTopLeft,
                NotificationPosition::TopRight => sys::ENotificationPosition::k_EPositionTopRight,
                NotificationPosition::BottomLeft => {
                    sys::ENotificationPosition::k_EPositionBottomLeft
                }
                NotificationPosition::BottomRight => {
                    sys::ENotificationPosition::k_EPositionBottomRight
                }
            };
            sys::SteamAPI_ISteamUtils_SetOverlayNotificationPosition(self.utils, position);
        }
    }

    /// Sets the Steam warning callback, which is called to emit warning messages.
    ///
    /// The passed-in function takes two arguments: a severity level (0 = info, 1 = warning) and
    /// the message itself.
    ///
    /// See [Steamwork's debugging page](https://partner.steamgames.com/doc/sdk/api/debugging) for more info.
    pub fn set_warning_callback<F>(&self, cb: F)
    where
        F: Fn(i32, &CStr) + Send + Sync + 'static,
    {
        let mut lock = WARNING_CALLBACK
            .write()
            .expect("warning func lock poisoned");
        *lock = Some(Box::new(cb));
        unsafe {
            sys::SteamAPI_ISteamUtils_SetWarningMessageHook(self.utils, Some(c_warning_callback));
        }
    }

    /// Gets the gamepad text input from the Big Picture overlay.
    ///
    /// This must be called within the `show_gamepad_text_input` callback. Returns `Some(String)` if user submitted the
    /// text, and `None` otherwise
    pub fn get_entered_gamepad_text_input(
        &self,
        dismissed_data: &GamepadTextInputDismissed,
    ) -> Option<String> {
        unsafe {
            let len = dismissed_data.submitted_text_len?;
            let mut buf = vec![0u8; len as usize];

            sys::SteamAPI_ISteamUtils_GetEnteredGamepadTextInput(
                self.utils,
                buf.as_mut_ptr().cast(),
                len,
            )
            .then(|| String::from_utf8(buf).expect("Steamworks returned invalid UTF-8 string"))
        }
    }

    /// Checks if Steam & the Steam Overlay are running in Big Picture mode.
    ///
    /// Games must be launched through the Steam client to enable the Big Picture overlay.
    /// During development, a game can be added as a non-steam game to the developer's library to test this feature.
    pub fn is_steam_in_big_picture_mode(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_IsSteamInBigPictureMode(self.utils) }
    }

    /// Checks if Steam is running on a Steam Deck device.
    pub fn is_steam_running_on_steam_deck(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_IsSteamRunningOnSteamDeck(self.utils) }
    }

    /// Activates the Big Picture text input dialog which only supports gamepad input.
    pub fn show_gamepad_text_input<F>(
        &self,
        input_mode: GamepadTextInputMode,
        input_line_mode: GamepadTextInputLineMode,
        description: &str,
        max_characters: u32,
        existing_text: Option<&str>,
        dismissed_cb: F,
    ) -> bool
    where
        F: FnMut(GamepadTextInputDismissed) + 'static + Send,
    {
        unsafe {
            let description = CString::new(description).unwrap();
            let existing_text = existing_text.map(|s| CString::new(s).unwrap());
            register_callback(&self._inner, dismissed_cb);
            sys::SteamAPI_ISteamUtils_ShowGamepadTextInput(
                self.utils,
                input_mode.into(),
                input_line_mode.into(),
                description.as_ptr(),
                max_characters,
                existing_text
                    .as_ref()
                    .map(|s| s.as_ptr())
                    .unwrap_or(std::ptr::null()),
            )
        }
    }

    /// Opens a floating keyboard over the game content and sends OS keyboard keys directly to the game.
    ///
    /// The text field position is specified in pixels relative the origin of the game window and is used to
    /// position the floating keyboard in a way that doesn't cover the text field.
    ///
    /// Callback is triggered when user dismisses the text input
    pub fn show_floating_gamepad_text_input<F>(
        &self,
        keyboard_mode: FloatingGamepadTextInputMode,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        mut dismissed_cb: F,
    ) -> bool
    where
        F: FnMut() + 'static + Send, // TODO: Support FnOnce callbacks
    {
        unsafe {
            register_callback(&self._inner, move |_: FloatingGamepadTextInputDismissed| {
                dismissed_cb();
            });
            sys::SteamAPI_ISteamUtils_ShowFloatingGamepadTextInput(
                self.utils,
                keyboard_mode.into(),
                x,
                y,
                width,
                height,
            )
        }
    }
}

impl Utils {
    /// Returns the number of seconds since the user was last active in this game.
    ///
    /// The SDK header does not define "active" any further; unverified against a live Steam client.
    pub fn seconds_since_app_active(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUtils_GetSecondsSinceAppActive(self.utils) }
    }

    /// Returns the number of seconds since the user was last active on this computer.
    ///
    /// The SDK header does not define "active" any further; unverified against a live Steam client.
    pub fn seconds_since_computer_active(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUtils_GetSecondsSinceComputerActive(self.utils) }
    }

    /// Returns the Steam universe this client connects to, or `None` when the SDK reports a value
    /// this binding does not name.
    pub fn connected_universe(&self) -> Option<Universe> {
        unsafe { Universe::from_sys(sys::SteamAPI_ISteamUtils_GetConnectedUniverse(self.utils)) }
    }

    /// Returns the battery charge in percent (0 to 100), or 255 while the computer is on AC power.
    pub fn current_battery_power(&self) -> u8 {
        unsafe { sys::SteamAPI_ISteamUtils_GetCurrentBatteryPower(self.utils) }
    }

    /// Returns the number of IPC calls made since the last call to this function.
    ///
    /// Each call resets the count. It measures how often the game talks to the Steam client;
    /// every IPC call costs at least a thread context switch.
    pub fn ipc_call_count(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUtils_GetIPCCallCount(self.utils) }
    }

    /// Returns whether the overlay needs the game to present a frame (`Present` or `SwapBuffers`).
    ///
    /// Only games that redraw on events instead of every frame need this. The SDK suggests polling
    /// it about 33 times a second and presenting when it returns `true`.
    pub fn overlay_needs_present(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_BOverlayNeedsPresent(self.utils) }
    }

    /// Starts an asynchronous check of a file against the signing set on the partner site, for
    /// example to refuse modified executables. The answer arrives through `run_callbacks()`.
    ///
    /// The callback receives `Err(SteamError::InvalidParameter)` when the name contains a NUL byte,
    /// and `Err(SteamError::IOFailure)` when the call fails. When the app has no signing set on the
    /// partner site, the verdict is `NoSignaturesFoundForThisApp`, not an error.
    pub fn check_file_signature<F>(&self, file_name: &str, cb: F)
    where
        F: FnOnce(Result<FileSignatureCheck, SteamError>) + 'static + Send,
    {
        let Ok(file_name) = CString::new(file_name) else {
            cb(Err(SteamError::InvalidParameter));
            return;
        };
        unsafe {
            let api_call =
                sys::SteamAPI_ISteamUtils_CheckFileSignature(self.utils, file_name.as_ptr());
            register_call_result::<sys::CheckFileSignature_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        FileSignatureCheck::from_sys(v.m_eCheckFileSignature)
                            .ok_or(SteamError::UnknownResult(v.m_eCheckFileSignature as i32))
                    })
                },
            );
        }
    }

    /// Returns the length of the text the user last entered in the gamepad text dialog.
    ///
    /// The SDK header does not state the unit or whether the terminator is counted; unverified
    /// against a live Steam client.
    pub fn entered_gamepad_text_length(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUtils_GetEnteredGamepadTextLength(self.utils) }
    }

    /// Returns whether Steam itself runs in VR mode.
    pub fn is_steam_running_in_vr(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_IsSteamRunningInVR(self.utils) }
    }

    /// Sets the inset, in pixels, of overlay notifications from the corner chosen with
    /// [`set_overlay_notification_position`](Self::set_overlay_notification_position).
    pub fn set_overlay_notification_inset(&self, horizontal: i32, vertical: i32) {
        unsafe {
            sys::SteamAPI_ISteamUtils_SetOverlayNotificationInset(self.utils, horizontal, vertical);
        }
    }

    /// Returns whether HMD content is streamed through Steam Remote Play.
    ///
    /// The default is `true` unless the game's extended app info sets `VRHeadsetStreaming` to `0`.
    pub fn is_vr_headset_streaming_enabled(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_IsVRHeadsetStreamingEnabled(self.utils) }
    }

    /// Sets whether HMD content is streamed through Steam Remote Play.
    ///
    /// `true` streams the headset scene and blocks remote input. `false` streams the application
    /// window and allows remote input, which suits games with asymmetric multiplayer.
    pub fn set_vr_headset_streaming_enabled(&self, enabled: bool) {
        unsafe { sys::SteamAPI_ISteamUtils_SetVRHeadsetStreamingEnabled(self.utils, enabled) }
    }

    /// Returns whether this Steam client is the Steam China launcher rather than the global client.
    pub fn is_steam_china_launcher(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_IsSteamChinaLauncher(self.utils) }
    }

    /// Loads the text filter dictionaries for the language the game runs in.
    ///
    /// Returns `false` when filtering is unavailable for that language. [`filter_text`](Self::filter_text)
    /// then passes text through unchanged. The SDK reserves the option flags, so they are always `0`.
    pub fn init_filter_text(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_InitFilterText(self.utils, 0) }
    }

    /// Filters a message with the legally required filter and the user's account preferences.
    ///
    /// Call [`init_filter_text`](Self::init_filter_text) first. `source` is the Steam ID of whoever
    /// wrote the text. Returns `Err(())` when the message contains a NUL byte.
    pub fn filter_text(
        &self,
        context: TextFilteringContext,
        source: SteamId,
        message: &str,
    ) -> Result<FilteredText, ()> {
        let message = CString::new(message).map_err(|_| ())?;
        // The SDK requires room for the input plus one byte. Four bytes per input byte also covers
        // any UTF-8 text the filter expands, so the output is not cut short.
        let mut buffer = vec![0 as c_char; message.as_bytes().len() * 4 + 1];
        let filtered_count = unsafe {
            sys::SteamAPI_ISteamUtils_FilterText(
                self.utils,
                context.into(),
                source.raw(),
                message.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
            )
        };
        Ok(FilteredText {
            text: string_from_c_buffer(&buffer),
            filtered_count,
        })
    }

    /// Returns what Steam believes the machine's IPv6 connectivity to the internet is, on one protocol.
    ///
    /// It does not tell whether the Steam client itself is connected over IPv6. Returns `None`
    /// when the SDK reports a value this binding does not name.
    pub fn ipv6_connectivity_state(&self, protocol: Ipv6Protocol) -> Option<Ipv6ConnectivityState> {
        unsafe {
            Ipv6ConnectivityState::from_sys(sys::SteamAPI_ISteamUtils_GetIPv6ConnectivityState(
                self.utils,
                protocol.into(),
            ))
        }
    }

    /// Turns on or off Steam Input translation of controller input into mouse and keyboard input,
    /// for game launchers that have no controller support of their own.
    pub fn set_game_launcher_mode(&self, enabled: bool) {
        unsafe { sys::SteamAPI_ISteamUtils_SetGameLauncherMode(self.utils, enabled) }
    }

    /// Dismisses the full-screen gamepad text input dialog.
    ///
    /// Returns the SDK's `bool` as it is; the header does not say what `false` means (unverified).
    /// The header does not say whether the callback given to `show_gamepad_text_input` runs after this (unverified).
    pub fn dismiss_gamepad_text_input(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_DismissGamepadTextInput(self.utils) }
    }

    /// Dismisses the floating keyboard. Returns the SDK's `bool` as it is; the header does not say
    /// what `false` means (unverified).
    pub fn dismiss_floating_gamepad_text_input(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUtils_DismissFloatingGamepadTextInput(self.utils) }
    }
}

pub(crate) struct SteamParamStringArray(Vec<*mut i8>);
impl Drop for SteamParamStringArray {
    fn drop(&mut self) {
        for c_string in &self.0 {
            unsafe { drop(CString::from_raw(*c_string)) };
        }
    }
}
impl SteamParamStringArray {
    pub(crate) fn new<S: AsRef<str>>(vec: &[S]) -> SteamParamStringArray {
        SteamParamStringArray(
            vec.into_iter()
                .map(|s| {
                    CString::new(s.as_ref())
                        .expect("String passed could not be converted to a c string")
                        .into_raw()
                })
                .collect(),
        )
    }

    pub(crate) fn as_raw(&mut self) -> sys::SteamParamStringArray_t {
        sys::SteamParamStringArray_t {
            m_nNumStrings: self.0.len() as i32,
            m_ppStrings: self.0.as_mut_ptr().cast(),
        }
    }
}
