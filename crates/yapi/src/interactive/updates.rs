//! The update checks interactive mode starts with, and their notices: pi's
//! `checkForNewPiVersion` (`utils/version-check.ts`) and
//! `checkForPackageUpdates` in `interactive-mode.ts`. yapi's releases are its
//! GitHub releases, and the notice names `yapi update`, which prints how to
//! upgrade.

use ratatui_core::style::Modifier;
use ratatui_core::text::{Line, Span};
use yapi_core::config::{APP_NAME, env_set};
use yapi_core::packages::PackageManager;
use yapi_tui::lines::{self, StyledLine, styled};

use super::chat::{Item, RenderContext};
use super::{App, Event};

impl App {
    /// Checks in the background for a newer yapi release and for package
    /// updates, and reports each that finds one as an event. Neither runs
    /// while `PI_OFFLINE` is set, which `--offline` does, and
    /// `PI_SKIP_VERSION_CHECK` turns off the release check.
    pub(super) fn check_for_updates(&self) {
        if env_set("PI_OFFLINE") {
            return;
        }
        if !env_set("PI_SKIP_VERSION_CHECK") {
            let tx = self.tx.clone();
            tokio::spawn(async move {
                if let Some(version) = new_version().await {
                    let _ = tx.send(Event::NewVersion(version));
                }
            });
        }
        let packages = PackageManager::new(
            self.session.project_dir(),
            self.agent_dir.clone(),
            self.session.settings_manager(),
            yapi_core::packages::npm::config(),
        );
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let updates = packages.available_updates().await;
            if !updates.is_empty() {
                let _ = tx.send(Event::PackageUpdates(updates));
            }
        });
    }
}

/// The latest release's version, when it is newer than this yapi: by semver
/// when both are versions, else whenever they differ, as pi's
/// `isNewerPackageVersion`.
async fn new_version() -> Option<String> {
    let releases = yapi_core::docs::releases_url();
    let latest = yapi_core::tools::external::latest_version(&releases)
        .await
        .ok()?;
    let current = env!("CARGO_PKG_VERSION");
    yapi_core::packages::npm::newer(&latest, current)
        .unwrap_or(latest.trim() != current)
        .then_some(latest)
}

/// pi's `showNewVersionNotification` for release `version`. A release
/// carries no note, and its changelog is on the releases page.
pub(super) fn version_notice(version: String) -> Item {
    let releases = yapi_core::docs::releases_url();
    Item::Render(Box::new(move |width, ctx| {
        let theme = ctx.theme;
        let accent = theme.fg("accent");
        let link = if ctx.hyperlinks {
            yapi_tui::ansi::link(accent, &releases)
        } else {
            accent
        };
        let text = [
            title("Update Available", ctx),
            Line::from(vec![
                Span::styled(
                    format!("New version {version} is available. Run "),
                    theme.fg("muted"),
                ),
                Span::styled(format!("{APP_NAME} update"), accent),
            ]),
            Line::from(vec![
                Span::styled("Changelog: ", theme.fg("muted")),
                Span::styled(releases.clone(), link),
            ]),
        ];
        framed(&text, width, ctx)
    }))
}

/// pi's `showPackageUpdateNotification` for the packages `names`.
pub(super) fn packages_notice(names: Vec<String>) -> Item {
    Item::Render(Box::new(move |width, ctx| {
        let theme = ctx.theme;
        let mut text = vec![
            title("Package Updates Available", ctx),
            Line::from(vec![
                Span::styled("Package updates are available. Run ", theme.fg("muted")),
                Span::styled(
                    format!("{APP_NAME} update --extensions"),
                    theme.fg("accent"),
                ),
            ]),
            styled("Packages:", theme.fg("muted")),
        ];
        text.extend(names.iter().map(|name| Line::raw(format!("- {name}"))));
        framed(&text, width, ctx)
    }))
}

fn title(text: &str, ctx: &RenderContext<'_>) -> StyledLine {
    styled(text, ctx.theme.fg("warning").add_modifier(Modifier::BOLD))
}

/// The layout of both notices: a spacer, then `text` between borders in
/// the warning color.
fn framed(text: &[StyledLine], width: usize, ctx: &RenderContext<'_>) -> Vec<StyledLine> {
    let border = lines::border(width, ctx.theme.fg("warning"));
    let mut out = lines::spacer(1);
    out.push(border.clone());
    out.extend(lines::text(text, width, 1, 0, None));
    out.push(border);
    out
}
