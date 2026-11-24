use super::*;
use std::time::Duration;

pub struct Timeline {
    pub(crate) timeline: *mut sys::ISteamTimeline,
    /// Whether the client's steam API is not recent enough.
    pub(crate) disabled: bool,
    pub(crate) _inner: Arc<Inner>,
}

pub enum TimelineGameMode {
    /// The player is fully loaded into the game and playing.
    Playing,
    /// The player is in a multiplayer lobby.
    Staging,
    /// The player is in the game's main menu or a pause menu.
    Menus,
    /// The player is waiting for a loading screen.
    LoadingScreen,
}

impl From<TimelineGameMode> for sys::ETimelineGameMode {
    fn from(mode: TimelineGameMode) -> Self {
        match mode {
            TimelineGameMode::Playing => sys::ETimelineGameMode::k_ETimelineGameMode_Playing,
            TimelineGameMode::Staging => sys::ETimelineGameMode::k_ETimelineGameMode_Staging,
            TimelineGameMode::Menus => sys::ETimelineGameMode::k_ETimelineGameMode_Menus,
            TimelineGameMode::LoadingScreen => {
                sys::ETimelineGameMode::k_ETimelineGameMode_LoadingScreen
            }
        }
    }
}

pub enum TimelineEventClipPriority {
    /// This event is not appropriate as a clip.
    None,
    /// The user may want to make a clip around this event.
    Standard,
    /// The player will be likely to want a clip around event,
    /// and those clips should be promoted more prominently than clips with the [TimelineEventClipPriority::Standard] priority.
    Featured,
}

impl From<TimelineEventClipPriority> for sys::ETimelineEventClipPriority {
    fn from(priority: TimelineEventClipPriority) -> Self {
        match priority {
            TimelineEventClipPriority::None => {
                sys::ETimelineEventClipPriority::k_ETimelineEventClipPriority_None
            }
            TimelineEventClipPriority::Standard => {
                sys::ETimelineEventClipPriority::k_ETimelineEventClipPriority_Standard
            }
            TimelineEventClipPriority::Featured => {
                sys::ETimelineEventClipPriority::k_ETimelineEventClipPriority_Featured
            }
        }
    }
}

impl Timeline {
    /// Changes the color of the timeline bar.
    pub fn set_timeline_game_mode(&self, mode: TimelineGameMode) {
        if self.disabled {
            return;
        }

        unsafe {
            sys::SteamAPI_ISteamTimeline_SetTimelineGameMode(self.timeline, mode.into());
        }
    }

    /// Sets a description for the current game state in the timeline.
    /// These help the user to find specific moments in the timeline when saving clips.
    /// Setting a new state description replaces any previous description.
    pub fn set_timeline_state_description(&self, description: &str, duration: Duration) {
        if self.disabled {
            return;
        }

        let description = CString::new(description).unwrap();

        unsafe {
            sys::SteamAPI_ISteamTimeline_SetTimelineTooltip(
                self.timeline,
                description.as_ptr(),
                duration.as_secs_f32(),
            )
        }
    }

    /// Clears the previous set game state in the timeline.
    pub fn clear_timeline_state_description(&self, duration: Duration) {
        if self.disabled {
            return;
        }

        unsafe {
            sys::SteamAPI_ISteamTimeline_ClearTimelineTooltip(self.timeline, duration.as_secs_f32())
        }
    }

    /// Use this to mark an event on the Timeline.
    /// The event can be instantaneous or take some amount of time to complete,
    /// depending on the value passed in `duration`.
    pub fn add_timeline_event(
        &self,
        icon: &str,
        title: &str,
        description: &str,
        priority: u32,
        start_offset_seconds: f32,
        duration: Duration,
        clip_priority: TimelineEventClipPriority,
    ) {
        if self.disabled {
            return;
        }

        let icon = CString::new(icon).unwrap();
        let title = CString::new(title).unwrap();
        let description = CString::new(description).unwrap();
        let duration = duration.as_secs_f32();

        unsafe {
            let _handle = sys::SteamAPI_ISteamTimeline_AddRangeTimelineEvent(
                self.timeline,
                icon.as_ptr(),
                title.as_ptr(),
                description.as_ptr(),
                priority,
                start_offset_seconds,
                duration,
                clip_priority.into(),
            );
        }
    }
}

/// The result of [`Timeline::does_event_recording_exist`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEventRecordingExists {
    /// The event handle that was asked about.
    pub event: u64,
    /// `true` when a background recording covers the event.
    pub recording_exists: bool,
}

/// The result of [`Timeline::does_game_phase_recording_exist`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineGamePhaseRecordingExists {
    /// The phase ID that was asked about, as Steam returns it.
    pub phase_id: String,
    /// Length in milliseconds of the background recording covering the phase.
    pub recording_ms: u64,
    /// Length in milliseconds of the longest clip made from the phase.
    pub longest_clip_ms: u64,
    /// Number of clips made from the phase.
    pub clip_count: u32,
    /// Number of screenshots taken during the phase.
    pub screenshot_count: u32,
}

/// Longest game phase ID in bytes. The SDK's `k_cchMaxPhaseIDLength` is 64 including the terminating NUL.
const MAX_PHASE_ID_BYTES: usize = 63;

/// Returns `Err(())` when `text` contains a NUL byte.
fn c_text(text: &str) -> Result<CString, ()> {
    CString::new(text).map_err(|_| ())
}

fn phase_id_text(phase_id: &str) -> Result<CString, ()> {
    if phase_id.len() > MAX_PHASE_ID_BYTES {
        return Err(());
    }
    c_text(phase_id)
}

/// Reads a NUL-terminated SDK text buffer. Bytes that are not UTF-8 are replaced.
fn text_from_buffer(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&byte| byte != 0)
        .map(|&byte| byte as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

impl Timeline {
    /// Adds an event that happens at one moment, `start_offset_seconds` from now (negative for the past). Returns its handle, or `None` when the timeline is unavailable or a text contains a NUL byte.
    ///
    /// `icon` is the name of an icon uploaded for the app or a built-in `steam_` icon. `title` and `description` must be in the language from `Utils::ui_language()`.
    /// The handle is valid only in this run of the game.
    pub fn add_instantaneous_timeline_event(
        &self,
        title: &str,
        description: &str,
        icon: &str,
        icon_priority: u32,
        start_offset_seconds: f32,
        clip_priority: TimelineEventClipPriority,
    ) -> Option<u64> {
        if self.disabled {
            return None;
        }
        let title = c_text(title).ok()?;
        let description = c_text(description).ok()?;
        let icon = c_text(icon).ok()?;
        Some(unsafe {
            sys::SteamAPI_ISteamTimeline_AddInstantaneousTimelineEvent(
                self.timeline,
                title.as_ptr(),
                description.as_ptr(),
                icon.as_ptr(),
                icon_priority,
                start_offset_seconds,
                clip_priority.into(),
            )
        })
    }

    /// Starts a range event now, plus `start_offset_seconds`. Returns its handle, or `None` when the timeline is unavailable or a text contains a NUL byte.
    ///
    /// The event must be ended with [`Timeline::end_range_timeline_event`]. Steam discards events still open when the game exits.
    pub fn start_range_timeline_event(
        &self,
        title: &str,
        description: &str,
        icon: &str,
        priority: u32,
        start_offset_seconds: f32,
        clip_priority: TimelineEventClipPriority,
    ) -> Option<u64> {
        if self.disabled {
            return None;
        }
        let title = c_text(title).ok()?;
        let description = c_text(description).ok()?;
        let icon = c_text(icon).ok()?;
        Some(unsafe {
            sys::SteamAPI_ISteamTimeline_StartRangeTimelineEvent(
                self.timeline,
                title.as_ptr(),
                description.as_ptr(),
                icon.as_ptr(),
                priority,
                start_offset_seconds,
                clip_priority.into(),
            )
        })
    }

    /// Changes the fields of a range event that was started and has not ended yet.
    ///
    /// Returns `Err(())` when a text contains a NUL byte. When the timeline is unavailable, nothing is sent and `Ok(())` is returned.
    pub fn update_range_timeline_event(
        &self,
        event: u64,
        title: &str,
        description: &str,
        icon: &str,
        priority: u32,
        clip_priority: TimelineEventClipPriority,
    ) -> Result<(), ()> {
        let title = c_text(title)?;
        let description = c_text(description)?;
        let icon = c_text(icon)?;
        if self.disabled {
            return Ok(());
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_UpdateRangeTimelineEvent(
                self.timeline,
                event,
                title.as_ptr(),
                description.as_ptr(),
                icon.as_ptr(),
                priority,
                clip_priority.into(),
            );
        }
        Ok(())
    }

    /// Ends a range event `end_offset_seconds` from now and shows it on the timeline. Does nothing when the timeline is unavailable.
    pub fn end_range_timeline_event(&self, event: u64, end_offset_seconds: f32) {
        if self.disabled {
            return;
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_EndRangeTimelineEvent(
                self.timeline,
                event,
                end_offset_seconds,
            );
        }
    }

    /// Removes an event from the timeline. Works for instantaneous and range events. Does nothing when the timeline is unavailable.
    pub fn remove_timeline_event(&self, event: u64) {
        if self.disabled {
            return;
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_RemoveTimelineEvent(self.timeline, event);
        }
    }

    /// Asks whether a background recording covers an event. `callback` runs exactly once, from `run_callbacks()`, with the answer.
    ///
    /// When the timeline is unavailable, `callback` runs at once with `Err(SteamError::Disabled)`.
    pub fn does_event_recording_exist<F>(&self, event: u64, callback: F)
    where
        F: FnOnce(Result<TimelineEventRecordingExists, SteamError>) + 'static + Send,
    {
        if self.disabled {
            callback(Err(SteamError::Disabled));
            return;
        }
        unsafe {
            let api_call =
                sys::SteamAPI_ISteamTimeline_DoesEventRecordingExist(self.timeline, event);
            register_call_result::<sys::SteamTimelineEventRecordingExists_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    callback(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        Ok(TimelineEventRecordingExists {
                            event: v.m_ulEventID,
                            recording_exists: v.m_bRecordingExists,
                        })
                    })
                },
            );
        }
    }

    /// Starts a game phase, which ends the previous one. A phase is a stretch of play of about 10 minutes to a few hours, such as one match or one chapter.
    ///
    /// Does nothing when the timeline is unavailable.
    pub fn start_game_phase(&self) {
        if self.disabled {
            return;
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_StartGamePhase(self.timeline);
        }
    }

    /// Ends the current game phase. Does nothing when the timeline is unavailable.
    pub fn end_game_phase(&self) {
        if self.disabled {
            return;
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_EndGamePhase(self.timeline);
        }
    }

    /// Sets the ID of the current game phase, so [`Timeline::open_overlay_to_game_phase`] and [`Timeline::does_game_phase_recording_exist`] can find it.
    ///
    /// Returns `Err(())` when the ID is longer than 63 bytes or contains a NUL byte. Does nothing when the timeline is unavailable.
    pub fn set_game_phase_id(&self, phase_id: &str) -> Result<(), ()> {
        let phase_id = phase_id_text(phase_id)?;
        if self.disabled {
            return Ok(());
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_SetGamePhaseID(self.timeline, phase_id.as_ptr());
        }
        Ok(())
    }

    /// Asks whether a background recording covers a game phase. `callback` runs exactly once, from `run_callbacks()`, with the answer.
    ///
    /// When the timeline is unavailable, `callback` runs at once with `Err(SteamError::Disabled)`. An ID that is too long or contains a NUL byte gets `Err(SteamError::InvalidParameter)` at once.
    pub fn does_game_phase_recording_exist<F>(&self, phase_id: &str, callback: F)
    where
        F: FnOnce(Result<TimelineGamePhaseRecordingExists, SteamError>) + 'static + Send,
    {
        if self.disabled {
            callback(Err(SteamError::Disabled));
            return;
        }
        let phase_id = match phase_id_text(phase_id) {
            Ok(phase_id) => phase_id,
            Err(()) => {
                callback(Err(SteamError::InvalidParameter));
                return;
            }
        };
        unsafe {
            let api_call = sys::SteamAPI_ISteamTimeline_DoesGamePhaseRecordingExist(
                self.timeline,
                phase_id.as_ptr(),
            );
            register_call_result::<sys::SteamTimelineGamePhaseRecordingExists_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    callback(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        Ok(TimelineGamePhaseRecordingExists {
                            phase_id: text_from_buffer(&v.m_rgchPhaseID),
                            recording_ms: v.m_ulRecordingMS,
                            longest_clip_ms: v.m_ulLongestClipMS,
                            clip_count: v.m_unClipCount,
                            screenshot_count: v.m_unScreenshotCount,
                        })
                    })
                },
            );
        }
    }

    /// Adds a tag that applies to the whole current game phase. Several tags of one group are all kept.
    ///
    /// `tag_name` and `tag_group` are localized text. `tag_icon` is an icon name. Returns `Err(())` when a text contains a NUL byte. Does nothing when the timeline is unavailable.
    pub fn add_game_phase_tag(
        &self,
        tag_name: &str,
        tag_icon: &str,
        tag_group: &str,
        priority: u32,
    ) -> Result<(), ()> {
        let tag_name = c_text(tag_name)?;
        let tag_icon = c_text(tag_icon)?;
        let tag_group = c_text(tag_group)?;
        if self.disabled {
            return Ok(());
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_AddGamePhaseTag(
                self.timeline,
                tag_name.as_ptr(),
                tag_icon.as_ptr(),
                tag_group.as_ptr(),
                priority,
            );
        }
        Ok(())
    }

    /// Sets a text attribute for the whole current game phase, such as a score. Only the last value set is shown.
    ///
    /// Returns `Err(())` when a text contains a NUL byte. Does nothing when the timeline is unavailable.
    pub fn set_game_phase_attribute(
        &self,
        attribute_group: &str,
        attribute_value: &str,
        priority: u32,
    ) -> Result<(), ()> {
        let attribute_group = c_text(attribute_group)?;
        let attribute_value = c_text(attribute_value)?;
        if self.disabled {
            return Ok(());
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_SetGamePhaseAttribute(
                self.timeline,
                attribute_group.as_ptr(),
                attribute_value.as_ptr(),
                priority,
            );
        }
        Ok(())
    }

    /// Opens the Steam overlay on a game phase that was given an ID with [`Timeline::set_game_phase_id`].
    ///
    /// Returns `Err(())` when the ID is too long or contains a NUL byte. Does nothing when the timeline is unavailable.
    pub fn open_overlay_to_game_phase(&self, phase_id: &str) -> Result<(), ()> {
        let phase_id = phase_id_text(phase_id)?;
        if self.disabled {
            return Ok(());
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_OpenOverlayToGamePhase(self.timeline, phase_id.as_ptr());
        }
        Ok(())
    }

    /// Opens the Steam overlay on a timeline event, given its handle. Does nothing when the timeline is unavailable.
    pub fn open_overlay_to_timeline_event(&self, event: u64) {
        if self.disabled {
            return;
        }
        unsafe {
            sys::SteamAPI_ISteamTimeline_OpenOverlayToTimelineEvent(self.timeline, event);
        }
    }
}
