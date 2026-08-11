//! How the window itself is opened and dressed.
//!
//! The window has no system title bar, because Subtext draws its own. That is
//! the same on every platform. The backdrop is not: Windows 11 can put a
//! blurred, desaturated wash of the desktop behind the window, and nothing else
//! can, so it is asked for and the answer is passed on rather than assumed.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Runtime, WebviewWindowBuilder};

use crate::dto::Failure;

/// What the front end needs to know about the window it is drawing into.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Chrome {
    /// The desktop shows through behind the window, so the surfaces drawn on
    /// top of it are the translucent ones. False everywhere the window is
    /// opaque, where the same surfaces would be washed out over nothing.
    pub(crate) backdrop: bool,
    /// Whether turning hardware decoding off would do anything here. Only the
    /// Windows webview takes a switch for it; everywhere else the decision
    /// belongs to the platform, and the settings screen leaves out a control it
    /// would not be telling the truth about.
    pub(crate) switchable_decoding: bool,
}

/// What the webview is told when it must not decode film on the graphics card.
///
/// Chromium takes its switches as one string, and passing any replaces the ones
/// the framework would have passed by itself, so both of those are repeated
/// here. The first turns off a menu and a reputation check that have no place
/// in a film player; the second is what lets a frame be captured for a poster
/// without somebody having pressed play first.
const SOFTWARE_DECODING: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
     --autoplay-policy=no-user-gesture-required --disable-accelerated-video-decode";

/// Opens the window this application is.
///
/// Built here rather than from the configuration alone, because how the webview
/// decodes video is settled when it is created and the answer is a preference
/// in the library file, which nothing has opened until now. Everything else
/// about the window is still the configuration's to say.
pub(crate) fn open<R: Runtime>(app: &AppHandle<R>, hardware_decoding: bool) -> Result<(), Failure> {
    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or_else(|| Failure::saying("this build has no window to open"))?
        .clone();

    let mut window = WebviewWindowBuilder::from_config(app, &config).map_err(Failure::of)?;
    if !hardware_decoding {
        window = window.additional_browser_args(SOFTWARE_DECODING);
    }
    window.build().map_err(Failure::of)?;

    Ok(())
}

/// Asks the platform for a backdrop, and says what the window turned out to be.
pub(crate) fn dress<R: Runtime>(app: &AppHandle<R>) -> Chrome {
    Chrome {
        backdrop: apply_backdrop(app),
        switchable_decoding: cfg!(windows),
    }
}

#[cfg(windows)]
fn apply_backdrop<R: Runtime>(app: &AppHandle<R>) -> bool {
    use tauri::Manager;
    use tauri::window::{Color, Effect, EffectsBuilder};

    let Some(window) = app.get_webview_window("main") else {
        return false;
    };

    // The dark variant rather than the one that follows the system, because
    // Subtext is dark whatever the desktop around it is doing, and a light Mica
    // behind a dark interface is a bright rim around the edge of the window.
    let applied = window.set_effects(EffectsBuilder::new().effect(Effect::MicaDark).build());
    if applied.is_err() {
        // Windows 10, or a build of 11 from before Mica. Nothing is wrong; the
        // window is simply opaque, which is what it looks like everywhere else.
        return false;
    }

    // Mica is drawn by the desktop compositor behind the window, so it is only
    // visible if the webview stops painting over it. Alpha of nothing is the
    // one translucent value Windows accepts here.
    window.set_background_color(Some(Color(0, 0, 0, 0))).is_ok()
}

#[cfg(not(windows))]
fn apply_backdrop<R: Runtime>(_app: &AppHandle<R>) -> bool {
    false
}
