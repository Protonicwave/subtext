//! How the window itself is dressed.
//!
//! The window has no system title bar, because Subtext draws its own. That is
//! the same on every platform. The backdrop is not: Windows 11 can put a
//! blurred, desaturated wash of the desktop behind the window, and nothing else
//! can, so it is asked for and the answer is passed on rather than assumed.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Runtime};

/// What the front end needs to know about the window it is drawing into.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Chrome {
    /// The desktop shows through behind the window, so the surfaces drawn on
    /// top of it are the translucent ones. False everywhere the window is
    /// opaque, where the same surfaces would be washed out over nothing.
    pub(crate) backdrop: bool,
}

/// Asks the platform for a backdrop, and says whether it gave one.
pub(crate) fn dress<R: Runtime>(app: &AppHandle<R>) -> Chrome {
    Chrome {
        backdrop: apply_backdrop(app),
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
