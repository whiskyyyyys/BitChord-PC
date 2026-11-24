//! The mini player: the same window, shrunk to a strip with the song, the transport and a
//! seek line, kept above other windows unless unpinned. Ctrl+M or the player bar's
//! button switches; the strip's expand button, its × (which closes the strip, not the app)
//! or a double-click switch back. With lyrics on, the window grows upward by a band the
//! strip's own height and shows the line being sung in it.
//!
//! Each mode keeps its own place: the full window goes back to exactly where it was
//! (maximized again if it was), and the strip reopens where it was last moved. Places
//! are kept in physical pixels with the monitor's scale, so they survive moving between
//! monitors with different scaling. Launching always opens the full window: closed in
//! the mini player, the app saves the full window's place for next time.
//!
//! On Windows the app draws the strip's window buttons itself (see `titlebar`). Elsewhere
//! the system's frame stays, with its own close button, so the strip leaves its out.
//! Keeping on top is up to the window manager (Wayland has no way to ask).

use std::time::Duration;

use egui::{Color32, Pos2, Rect, Sense, Ui, Vec2, ViewportCommand, WindowLevel, pos2, vec2};
use serde::{Deserialize, Serialize};

use super::widgets::{self, icon_at, named, paint_cover, text_at};
use super::{Action, App, fmt_time, glyphs, titlebar};
use crate::theme::{self, icon};

// Decisions that are easy to change.

/// The strip when first opened. It's as tall as its contents; the width goes from
/// `MIN_W` to `MAX_W` by its sides.
pub const SIZE: Vec2 = vec2(400.0, 112.0);
/// The line being sung, when lyrics are on: a band above the strip the strip's own size,
/// so the window doubles in height and the line gets the strip's whole width.
pub const LYR_H: f32 = SIZE.y;
const MIN_W: f32 = 330.0;
const MAX_W: f32 = 720.0;
/// Kept above other windows until unpinned.
const PINNED_BY_DEFAULT: bool = true;
/// From the work area's corner, the first time.
const MARGIN: f32 = 16.0;

/// The window's height: the strip, with the lyric band over it when lyrics are on.
fn height(lyrics: bool) -> f32 {
    if lyrics { SIZE.y + LYR_H } else { SIZE.y }
}

/// A strip button's glyph: from the icon font, or one of Apple's own drawn ones.
enum Btn {
    Font(char),
    Raster(&'static [u8]),
}

/// Where a window was: its outer corner in physical pixels, its inner size in points,
/// and the monitor's scale and the app's zoom then (points are that big).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub scale: f32,
    #[serde(default = "unzoomed")]
    pub zoom: f32,
}

fn unzoomed() -> f32 {
    1.0
}

impl Place {
    /// The window's place now, once the platform has said. Wayland never tells apps
    /// where their windows are (nor lets them move them): there only the size counts.
    fn current(ctx: &egui::Context) -> Option<Place> {
        let ppp = ctx.pixels_per_point();
        let size = ctx.content_rect().size();
        let zoom = ctx.zoom_factor();
        ctx.input(|i| {
            let vp = i.viewport();
            let pos = vp.outer_rect.map_or(Pos2::ZERO, |r| r.min * ppp);
            Some(Place { x: pos.x, y: pos.y, w: size.x, h: size.y, scale: vp.native_pixels_per_point?, zoom })
        })
    }

    fn size(&self) -> Vec2 {
        vec2(self.w, self.h)
    }

    /// Its size in points at the app's zoom now (it may have changed since).
    fn size_at(&self, zoom: f32) -> Vec2 {
        self.size() * (self.zoom / zoom)
    }

    /// The outer rectangle in physical pixels, given the frame around the inside (points).
    fn rect_px(&self, frame: Vec2) -> Rect {
        Rect::from_min_size(pos2(self.x, self.y), (self.size() + frame) * self.scale * self.zoom)
    }

    /// eframe's saved-window format, so a launch after closing in the mini player opens
    /// the full window where it was.
    fn eframe_window(&self, maximized: bool) -> String {
        format!(
            "(normal_geometry:(outer_position_pixels:Some((x:{:?},y:{:?})),scale_factor:{:?},inner_size_points:(x:{:?},y:{:?})),fullscreen:false,maximized:{maximized})",
            self.x, self.y, self.scale, self.w, self.h
        )
    }
}

/// The strip's first place: the bottom right of `area` (physical pixels), clear of its
/// edges, for a window with `frame` (points) around its inside. A point is `scale`
/// times `zoom` pixels.
fn corner_place(area: Rect, frame: Vec2, scale: f32, zoom: f32) -> Place {
    let outer = (SIZE + frame) * scale * zoom;
    let p = area.max - outer - Vec2::splat(MARGIN * scale);
    let p = p.max(area.min);
    Place { x: p.x.round(), y: p.y.round(), w: SIZE.x, h: SIZE.y, scale, zoom }
}

/// A window `size` (points) in the middle of `area` (physical pixels), kept inside it.
fn centered(area: Rect, size: Vec2, frame: Vec2, scale: f32, zoom: f32) -> Place {
    let outer = (size + frame) * scale * zoom;
    let p = (area.center() - outer / 2.0).max(area.min);
    Place { x: p.x.round(), y: p.y.round(), w: size.x, h: size.y, scale, zoom }
}

/// The frame around the window's inside (points): what the platform drew outside the
/// content. Nothing on Windows, where the app draws its own buttons.
fn frame_size(ctx: &egui::Context) -> Vec2 {
    ctx.input(|i| {
        let vp = i.viewport();
        vp.outer_rect.zip(vp.inner_rect).map_or(Vec2::ZERO, |(o, n)| o.size() - n.size())
    })
}

/// What the app can ask about the monitors (the system's; made up in tests).
#[derive(Clone, Copy)]
struct Screens {
    /// Whether some of a rectangle (physical pixels) is on a connected monitor.
    on_a_monitor: fn(Rect) -> bool,
    /// The window's monitor less the taskbar, in physical pixels.
    work_area: fn() -> Option<Rect>,
}

const SYSTEM: Screens = Screens { on_a_monitor: crate::fullscreen::on_a_monitor, work_area: crate::fullscreen::work_area };

/// A remembered strip size, within the limits (they may have changed since). The strip
/// has one height; with lyrics it has the lyric band's too.
fn clamp_size(p: Place, lyrics: bool) -> Place {
    Place { w: p.w.clamp(MIN_W, MAX_W), h: height(lyrics), ..p }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    /// Waiting for a maximized full window to come down, so its normal place is known.
    Unmaximize,
    /// Move and resize now.
    Resize,
    /// Moved onto a monitor with another scale: size it once the window has that scale.
    Rescale(Place),
    /// Waiting for the window to reach its new size (None: to be maximized) before the
    /// new layout shows.
    Settle(Option<Vec2>),
}

struct Switch {
    to_mini: bool,
    step: Step,
    since: f64,
    /// The window's size when it began.
    from: Vec2,
}

/// What to draw this frame.
#[derive(Clone, Copy, PartialEq)]
pub enum Layout {
    Full,
    Mini,
    /// Just the background: mid-switch, the window no longer fits the old layout and
    /// doesn't have the new one's size yet.
    Blank,
}

/// The band's page of the lyric: the line it shows, the one it came from, and how far
/// through it the singing is.
#[derive(Default)]
struct Band {
    /// The song these belong to. A new one takes the band without a hand-over.
    id: String,
    /// Where in the lyric the shown line is.
    index: usize,
    cur: Option<Shot>,
    /// The line before `cur`, when it started to leave, and how far the singing had got it.
    out: Option<(Shot, f64, f32)>,
    /// When `cur` came in.
    at: f64,
    /// Where the singing is, in milliseconds with the line's offset applied.
    t: f64,
}

/// One lyric line as the band draws it.
struct Shot {
    text: String,
    /// `(first char, chars, start ms, dur ms)` for each timed word. A source that only
    /// times lines gets one entry over the whole line, which then lights whole.
    words: Vec<(usize, usize, f64, f64)>,
}

/// The line being sung, with its place in the lyric and the time it is sung at.
struct Cue {
    id: String,
    index: usize,
    shot: Shot,
    t: f64,
}

/// How long a line takes to rise away and its successor to rise in.
const RISE: f64 = 0.36;

pub struct Mini {
    /// The strip is showing.
    on: bool,
    pub pinned: bool,
    /// The strip's last place.
    place: Option<Place>,
    /// The full window's, from before it shrank, and whether it was maximized.
    full: Option<(Place, bool)>,
    was_maximized: bool,
    switch: Option<Switch>,
    /// A switch asked for, started once the window is ready.
    wanted: Option<bool>,
    /// When the current layout appeared, for its fade-in.
    shown_at: f64,
    /// The scale the size limits were last sent at: winit keeps them in pixels.
    limits_scale: Option<f32>,
    /// The window level last sent, and when: a topmost strip is re-stamped now and
    /// then, since another window's z-order change can knock it down.
    level: Option<WindowLevel>,
    level_at: f64,
    screens: Screens,
    /// The full window's normal place after coming back maximized, until it's restored down.
    restored_max: Option<Place>,
    /// The strip has the lyric band above it. Session only; it moves the strip, it
    /// doesn't switch to the full window.
    pub lyrics: bool,
    /// What the band button was last asked for. The band itself only shows when the song
    /// has lyrics to fill it, so an ask for a song without any waits for the next one.
    pub lyrics_want: bool,
    /// The band's line, the one it came from, and where in the line the singer is.
    band: Band,
}

impl Mini {
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Self {
        Mini {
            on: false,
            pinned: get("mini_pinned").map_or(PINNED_BY_DEFAULT, |v| v == "1"),
            place: get("mini_window").and_then(|v| serde_json::from_str(&v).ok()).map(|p| clamp_size(p, false)),
            full: None,
            was_maximized: false,
            switch: None,
            wanted: None,
            shown_at: f64::NEG_INFINITY,
            limits_scale: None,
            level: None,
            level_at: f64::NEG_INFINITY,
            screens: SYSTEM,
            restored_max: None,
            lyrics: false,
            lyrics_want: false,
            band: Band::default(),
        }
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string("mini_pinned", if self.pinned { "1" } else { "0" }.into());
        if let Some(p) = &self.place {
            storage.set_string("mini_window", serde_json::to_string(p).unwrap_or_default());
        }
    }

    /// Ask for the strip (true) or the full window; started by [`Mini::take_request`].
    pub fn request(&mut self, on: bool) {
        self.wanted = Some(on);
    }

    /// The switch to start now, if any (not while one runs).
    pub fn take_request(&mut self) -> Option<bool> {
        if self.switch.is_some() {
            return None;
        }
        self.wanted.take()
    }

    /// Where it's headed once queued switches are done: what Ctrl+M flips.
    pub fn target(&self) -> bool {
        self.wanted.unwrap_or_else(|| self.active())
    }

    /// The strip is showing or on its way.
    pub fn active(&self) -> bool {
        self.switch.as_ref().map_or(self.on, |s| s.to_mini)
    }

    /// The strip is showing, or the window is switching either way: the full window's
    /// pages and now playing aren't up.
    pub fn busy(&self) -> bool {
        self.on || self.switch.is_some()
    }

    /// The strip is showing (not mid-switch).
    pub fn showing(&self) -> bool {
        self.on && self.switch.is_none()
    }

    /// The old layout stays until the window starts changing size.
    pub fn layout(&self, ctx: &egui::Context) -> Layout {
        match &self.switch {
            Some(sw) if (ctx.content_rect().size() - sw.from).length() > 1.0 => Layout::Blank,
            _ if self.on => Layout::Mini,
            _ => Layout::Full,
        }
    }

    /// The full window's place in eframe's format, while eframe's own would be wrong: with
    /// the strip up or on its way back (eframe's is the strip's), and while the full window
    /// is still maximized after it (eframe took the strip's as its normal size).
    pub fn full_window(&self) -> Option<String> {
        if self.active() || self.switch.is_some() {
            return self.full.map(|(p, max)| p.eframe_window(max));
        }
        self.restored_max.map(|p| p.eframe_window(true))
    }

    /// The new layout fades in from `base` (the background's own colour): drawn over
    /// everything, since pages and popups paint on layers of their own.
    pub fn fade_in(&self, ctx: &egui::Context, now: f64, base: Color32) {
        let t = ((now - self.shown_at) / 0.18).clamp(0.0, 1.0) as f32;
        if t >= 1.0 {
            return;
        }
        ctx.request_repaint();
        let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("mini-fade"));
        ctx.layer_painter(layer).rect_filled(ctx.content_rect(), 0.0, base.gamma_multiply(1.0 - theme::ease_out(t)));
    }

    /// Switch to the strip or back. The full window leaves full screen first (the caller's).
    pub fn toggle(&mut self, ctx: &egui::Context, now: f64) {
        if self.switch.is_some() {
            return;
        }
        let to_mini = !self.on;
        let mut step = Step::Resize;
        if to_mini {
            self.was_maximized = ctx.input(|i| i.viewport().maximized == Some(true));
            if self.was_maximized {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
                step = Step::Unmaximize;
            }
        }
        self.switch = Some(Switch { to_mini, step, since: now, from: ctx.content_rect().size() });
        ctx.request_repaint();
    }

    pub fn set_pinned(&mut self, ctx: &egui::Context, on: bool) {
        self.pinned = on;
        self.apply_level(ctx);
    }

    /// Push the level the current mode asks for. A pinned strip is re-stamped every
    /// couple of seconds: raising another window can drop it out of the topmost band.
    fn apply_level(&mut self, ctx: &egui::Context) {
        let want = if self.active() && self.pinned { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal };
        let now = ctx.input(|i| i.time);
        let since = now - self.level_at;
        let stale = want == WindowLevel::AlwaysOnTop && since > 2.0;
        if self.level == Some(want) && !stale {
            // Wake for the next re-stamp even when nothing else is animating.
            if want == WindowLevel::AlwaysOnTop {
                ctx.request_repaint_after(Duration::from_secs_f64((2.0 - since).max(0.2)));
            }
            return;
        }
        self.level = Some(want);
        self.level_at = now;
        // Windows stamps the style itself: winit posts its change to the window's thread
        // (`SWP_ASYNCWINDOWPOS`) and can lose it, which is what made the pin look dead.
        // Elsewhere that is the only way.
        if !crate::fullscreen::set_topmost(want == WindowLevel::AlwaysOnTop) {
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(want));
        }
    }

    /// Once a frame: drive a switch, note the strip's place, keep the size limits right.
    pub fn tick(&mut self, ctx: &egui::Context, now: f64) {
        let scale = ctx.input(|i| i.viewport().native_pixels_per_point);
        // winit keeps the limits in pixels: again after a move to another scale or a zoom.
        if self.limits_scale.is_some_and(|s| s != ctx.pixels_per_point()) && self.switch.is_none() {
            self.limits(ctx, self.on);
        }
        if self.restored_max.is_some() && self.switch.is_none() && ctx.input(|i| i.viewport().maximized == Some(false)) {
            self.restored_max = None;
        }
        // The strip keeps the level its pin asks for; the full window never stays on top.
        self.apply_level(ctx);
        let (minimized, maximized) = ctx.input(|i| (i.viewport().minimized == Some(true), i.viewport().maximized == Some(true)));
        // Win+Up or a drag onto the screen's top edge: the strip has one height.
        if self.showing() && maximized && !minimized {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
        }
        // Minimized, the platform says no place (and the strip's isn't one maximized).
        if self.showing()
            && !minimized
            && !maximized
            && let Some(p) = Place::current(ctx).filter(|p| p.w >= MIN_W - 1.0)
        {
            self.place = Some(clamp_size(p, self.lyrics));
        }
        let Some(sw) = &mut self.switch else { return };
        ctx.request_repaint_after(Duration::from_millis(16));
        let waited = now - sw.since;
        let to_mini = sw.to_mini;
        let next = match sw.step {
            Step::Unmaximize => {
                let down = ctx.input(|i| i.viewport().maximized != Some(true));
                // Its normal place, at once: closing before the next step still saves it.
                if down {
                    self.full = Place::current(ctx).map(|p| (p, true));
                }
                (down || waited > 0.5).then_some(Step::Resize)
            }
            Step::Resize => Some(self.resize(ctx, to_mini)),
            Step::Rescale(target) => (scale.is_some_and(|s| (s - target.scale).abs() < 0.01) || waited > 0.6).then(|| {
                self.limits(ctx, to_mini);
                let size = target.size_at(ctx.zoom_factor());
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
                Step::Settle(Some(size))
            }),
            Step::Settle(size) => {
                let done = match size {
                    Some(size) => (ctx.content_rect().size() - size).length() < 2.0,
                    None => ctx.input(|i| i.viewport().maximized == Some(true)),
                };
                if done || waited > 0.4 {
                    log::info!("{}: {:?}{}", if to_mini { "mini player" } else { "full window" }, Place::current(ctx), if done { "" } else { " (timed out)" });
                    self.on = to_mini;
                    if !to_mini {
                        self.restored_max = self.full.take().filter(|(_, max)| *max).map(|(p, _)| p);
                    }
                    self.switch = None;
                    self.shown_at = now;
                    return;
                }
                None
            }
        };
        if let (Some(step), Some(sw)) = (next, &mut self.switch) {
            sw.step = step;
            sw.since = now;
        }
    }

    /// Whether a place is (at least partly) on a connected monitor. Test runs live off-screen.
    fn visible(&self, p: Place, frame: Vec2) -> bool {
        // Its top middle, where it can be grabbed: a sliver on screen isn't enough.
        let r = p.rect_px(frame);
        let grip = Rect::from_center_size(pos2(r.center().x, r.top() + 12.0 * p.scale), vec2(r.width().min(120.0 * p.scale), 16.0 * p.scale));
        std::env::var_os("YTM_BACKGROUND").is_some() || (self.screens.on_a_monitor)(grip)
    }

    /// Move and size the window for the new mode; the step to wait in after.
    fn resize(&mut self, ctx: &egui::Context, to_mini: bool) -> Step {
        let now_place = Place::current(ctx);
        let frame = ctx.input(|i| {
            let vp = i.viewport();
            vp.outer_rect.zip(vp.inner_rect).map_or(Vec2::ZERO, |(o, i)| o.size() - i.size())
        });
        if to_mini {
            self.full = now_place.map(|p| (p, self.was_maximized));
            let target = match (self.place, now_place) {
                // Test runs stay off-screen, where they are; YTM_MINI_W tries other widths.
                (_, Some(here)) if std::env::var_os("YTM_BACKGROUND").is_some() => {
                    let w = std::env::var("YTM_MINI_W").ok().and_then(|w| w.parse().ok()).unwrap_or(SIZE.x);
                    clamp_size(Place { w, ..here }, self.lyrics)
                }
                (Some(p), _) if self.visible(p, frame) => p,
                (_, here) => {
                    let scale = here.map_or(1.0, |h| h.scale);
                    // Not Windows: the bottom right of the full window, where the player bar was.
                    let area = (self.screens.work_area)()
                        .or_else(|| here.map(|h| h.rect_px(frame)))
                        .unwrap_or(Rect::from_min_size(Pos2::ZERO, SIZE * scale));
                    corner_place(area, frame, scale, ctx.zoom_factor())
                }
            };
            self.apply_level(ctx);
            self.go(ctx, target, false, true)
        } else {
            if let Some(p) = now_place {
                self.place = Some(clamp_size(p, self.lyrics));
            }
            self.apply_level(ctx);
            // Kept until the window has settled: a save meanwhile still writes it.
            match self.full {
                Some((p, max)) if self.visible(p, frame) => self.go(ctx, p, max, false),
                // Its monitor is gone: its size, in the middle of the strip's monitor.
                full => {
                    let zoom = ctx.zoom_factor();
                    let size = full.map_or(Vec2::from(crate::DEFAULT_SIZE), |(p, _)| p.size_at(zoom));
                    let max = full.is_some_and(|(_, max)| max);
                    match ((self.screens.work_area)(), now_place) {
                        (Some(area), Some(here)) => {
                            // Its new place is the one to save (and restore down to).
                            let target = centered(area, size, frame, here.scale, zoom);
                            self.full = Some((target, max));
                            self.go(ctx, target, max, false)
                        }
                        _ => {
                            self.limits(ctx, false);
                            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
                            Step::Settle(Some(size))
                        }
                    }
                }
            }
        }
    }

    /// Move first, then the limits (changing them can resize the window at once, which
    /// could carry it onto another monitor before the move), then the size.
    fn go(&mut self, ctx: &egui::Context, target: Place, maximize: bool, mini: bool) -> Step {
        let ppp = ctx.pixels_per_point();
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(target.x, target.y) / ppp));
        self.limits(ctx, mini);
        let scale = ctx.input(|i| i.viewport().native_pixels_per_point);
        if scale.is_some_and(|s| (s - target.scale).abs() >= 0.01) && !maximize {
            // Points mean another size there: wait for the move to land.
            return Step::Rescale(target);
        }
        let size = target.size_at(ctx.zoom_factor());
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        if maximize {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
            return Step::Settle(None);
        }
        Step::Settle(Some(size))
    }

    /// The window's size limits for a mode. Shrinking sets them first, so the full
    /// window's minimum doesn't hold the strip up. With lyrics the strip carries its band,
    /// so the height is fixed at both and only the width runs.
    fn limits(&mut self, ctx: &egui::Context, mini: bool) {
        if mini {
            let h = height(self.lyrics);
            ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(vec2(MIN_W, h)));
            ctx.send_viewport_cmd(ViewportCommand::MaxInnerSize(vec2(MAX_W, h)));
        } else {
            ctx.send_viewport_cmd(ViewportCommand::MaxInnerSize(Vec2::INFINITY));
            ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(Vec2::from(crate::MIN_SIZE)));
        }
        self.limits_scale = Some(ctx.pixels_per_point());
    }

    /// Open or close the lyric band. The window grows upward by the band and shrinks back
    /// the same way, so the strip itself never moves: the strip's and the full window's
    /// places are both kept, so this isn't a switch between modes.
    pub fn set_lyrics(&mut self, ctx: &egui::Context, on: bool) {
        if self.lyrics == on || self.switch.is_some() {
            return;
        }
        self.lyrics = on;
        let frame = frame_size(ctx);
        let here = Place::current(ctx).or(self.place).unwrap_or_else(|| {
            let area = (self.screens.work_area)().unwrap_or(Rect::from_min_size(Pos2::ZERO, SIZE * ctx.pixels_per_point()));
            corner_place(area, frame, ctx.pixels_per_point(), ctx.zoom_factor())
        });
        // Physical pixels: the band is a fixed height in points.
        let band = LYR_H * here.scale * here.zoom;
        let mut target = clamp_size(Place { y: here.y + if on { -band } else { band }, ..here }, on);
        if let Some(area) = (self.screens.work_area)() {
            // Taller or shorter than before: keep its grip on the monitor.
            let outer = (target.size() + frame) * target.scale * target.zoom;
            let keep = |p: f32, size: f32, a: f32, b: f32| p.clamp(a.max(b - size), b.max(a));
            target.x = keep(target.x, outer.x, area.min.x, area.max.x);
            target.y = keep(target.y, outer.y, area.min.y, area.max.y);
        }
        self.place = Some(target);
        self.limits(ctx, true);
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(target.x, target.y) / ctx.pixels_per_point()));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(target.size_at(ctx.zoom_factor())));
        ctx.request_repaint();
    }

    /// The band's button: asks for the band, or takes the ask back. Whether the window
    /// actually grows is [`Mini::sync_lyrics`]'s call — a song with nothing timed has no
    /// band to show, and the ask carries over to the next one that does.
    pub fn toggle_lyrics(&mut self) {
        self.lyrics_want = !self.lyrics_want;
    }

    /// Keep the window's shape in step with what the song can fill the band with: a song
    /// without lyrics drops back to the plain strip, and the band returns by itself for
    /// the next song that has them.
    pub fn sync_lyrics(&mut self, ctx: &egui::Context, available: bool) {
        let on = self.lyrics_want && available;
        if on != self.lyrics {
            self.set_lyrics(ctx, on);
        }
    }
}

/// The strip: cover, title and artists, like / previous / play / next / volume, a seek
/// line along the bottom, full window / lyrics / close at the top right, and the pin alone
/// at the top left.
pub fn strip(app: &mut App, ui: &mut Ui, now: f64) {
    let ctx = ui.ctx().clone();
    let r = ctx.content_rect();
    // The corners the buttons live in: a press there is the button's, never the window's.
    // A strip that slid away under the pointer turns the click into a drag and loses it,
    // which is how the pin came to look like a button that moves the player.
    let count = if titlebar::custom() { 3.0 } else { 2.0 };
    let corners = [
        Rect::from_min_max(r.min + vec2(2.0, 2.0), r.min + vec2(38.0, 38.0)),
        Rect::from_min_max(pos2(r.right() - 8.0 - 30.0 * count, r.top()), pos2(r.right(), r.top() + 40.0)),
    ];
    // Its background moves the window; a double-click goes back to the full window.
    if titlebar::drag_strip(ui, r, "mini", &corners).double_clicked() {
        app.actions.push(Action::Mini(false));
    }
    ui.painter().rect_filled(r, theme::thumb_radius() + 1.0, theme::panel(0.62));

    // An Automix handoff keeps the outgoing song here until the next one is louder.
    let hold = app.mix_hold().is_some();
    let track = if hold { app.shown().cloned() } else { app.current().cloned() };
    let pad = 12.0;
    // With lyrics on, the window's top band belongs to the line being sung and everything
    // under it is the strip as it draws on its own. A song with nothing timed has no band
    // to show, so the ask waits for the next one that does.
    let timed = !app.no_lyrics();
    app.mini.sync_lyrics(&ctx, timed);
    let side = app.mini.lyrics;
    let want = app.mini.lyrics_want;
    let band = if side { Rect::from_min_max(r.min, pos2(r.right(), r.top() + LYR_H)) } else { r };
    let col = if side { Rect::from_min_max(pos2(r.left(), band.bottom()), r.max) } else { r };
    let art = Rect::from_min_size(col.min + Vec2::splat(pad), Vec2::splat(col.height() - 2.0 * pad - 4.0));
    match &track {
        Some(t) => paint_cover(ui, art, &t.thumbs, theme::thumb_radius() + 1.0, 1.0),
        None => {
            ui.painter().rect_filled(art, theme::thumb_radius() + 1.0, theme::glass(0.08));
            icon_at(ui, art.center(), icon::MUSIC, 28.0, theme::text_faint());
        }
    }
    // Apple's motion sleeve, over the still until its first frame arrives.
    if let Some(t) = &track
        && let Some(album) = t.album.as_ref().map(|a| a.name.clone()).filter(|a| !a.is_empty())
    {
        let artist = t.artists.first().map(|a| a.name.clone()).unwrap_or_else(|| t.artist_line());
        let px = (art.width() * ctx.pixels_per_point()).round().max(256.0) as u32;
        app.apple_cover(&album, &artist, Some(&t.title));
        if let Some(s) = app.apple_sprite(&ctx, &album, &artist, Some(&t.title), px) {
            widgets::paint_motion(ui, art, theme::thumb_radius() + 1.0, &s);
        }
    }

    // Title and artists.
    let x0 = art.right() + 12.0;
    let x1 = col.right() - 10.0;
    let text_w = (x1 - x0).max(20.0);
    let ty = art.top() + 1.0;
    let (title, artists) = match &track {
        Some(t) => (t.title.clone(), t.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")),
        None => ("Nothing playing".to_owned(), String::new()),
    };
    let tr = text_at(ui, pos2(x0, ty), &title, theme::semibold(14.5), theme::text(), text_w);
    if let Some(t) = &track {
        named(ui.interact(tr, ui.scope_id().with("mini-title"), Sense::hover()), &t.title).on_hover_text(&t.title);
    }
    let ar = text_at(ui, pos2(x0, ty + 21.0), &artists, theme::regular(12.5), theme::text_dim(), text_w);
    // The quality glyph follows the artists, centred on their line, when there's room.
    let bw = super::nowplaying::audio_badge_icon_width(app, true);
    if bw > 0.0 && ar.right() + 8.0 + bw <= x1 {
        super::nowplaying::audio_badge_icon(app, ui, pos2(ar.right() + 8.0 + bw / 2.0, ar.center().y), true);
    }

    // Top right: full window, lyrics and close. They show while the pointer is on the strip
    // (or one of them has the keyboard), over a backdrop, so the title has the whole width.
    // The pin has its own corner — see below.
    let scope = ui.scope_id();
    // By a key, not the label: the pin's changes, and its focus has to stay.
    let btn_id = move |key: &str| scope.with(("mini-btn", key));
    let focused = ctx.memory(|m| m.focused()).is_some_and(|f| ["close", "full", "pin", "lyrics"].iter().any(|k| btn_id(k) == f));
    let over = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| r.contains(p));
    let reveal = theme::anim_bool(&ctx, "mini-btns", over || focused, 0.15);
    let backdrop = Rect::from_min_max(pos2(r.right() - 6.0 - 30.0 * count - 2.0, r.top() + 2.0), pos2(r.right() - 2.0, r.top() + 38.0));
    // Opaque, so the title under it fades out instead of showing through.
    let pill = theme::lerp_color(super::shell::base_color(app), if theme::light() { Color32::WHITE } else { Color32::BLACK }, 0.55);
    ui.painter().rect_filled(backdrop, 9.0, pill.gamma_multiply(reveal));
    let mut bx = r.right() - 6.0;
    let mut top_btn = |ui: &mut Ui, glyph: Btn, key: &str, name: &str, close: bool, on: bool| -> bool {
        let b = Rect::from_min_size(pos2(bx - 28.0, r.top() + 6.0), Vec2::splat(28.0));
        bx -= 30.0;
        let resp = ui.interact(b, btn_id(key), Sense::click());
        let resp = named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand);
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.12);
        let down = resp.is_pointer_button_down_on();
        if h > 0.0 {
            let fill = if close { Color32::from_rgb(0xC4, 0x2B, 0x1C).gamma_multiply(h) } else { theme::glass(if down { 0.16 } else { 0.10 } * h) };
            ui.painter().rect_filled(b, 6.0, fill.gamma_multiply(reveal));
        }
        // Everything fades with the pointer, whether or not the button is engaged: an
        // engaged one says so by its colour while the pointer is on it.
        let show = reveal;
        let idle = if on { theme::text() } else { theme::text_dim() };
        let fg = theme::lerp_color(idle, if close { Color32::WHITE } else { theme::text() }, h);
        let size = if down { 13.0 } else { 14.0 };
        match glyph {
            Btn::Font(c) => {
                icon_at(ui, b.center(), c, size, fg.gamma_multiply(show));
            }
            Btn::Raster(p) => glyphs::draw(ui, b.center(), 1.2 * size, p, fg.gamma_multiply(show)),
        }
        if resp.has_focus() {
            ui.painter().rect_stroke(b.shrink(1.0), 6.0, egui::Stroke::new(1.5, theme::text()), egui::StrokeKind::Inside);
        }
        // A click that landed on the button counts even before the reveal finishes;
        // only a press the drag area already claimed stays a window move.
        resp.clicked() && (show > 0.5 || resp.has_focus() || resp.hovered())
    };
    // The system's frame has its own close button. The strip's × closes the strip: back to
    // the full window, never the app.
    if titlebar::custom() && top_btn(ui, Btn::Font(icon::X), "close", "Close mini player", true, false) {
        app.actions.push(Action::Mini(false));
    }
    if top_btn(ui, Btn::Font(icon::MAXIMIZE_2), "full", "Full window (Ctrl+M)", false, false) {
        app.actions.push(Action::Mini(false));
    }
    if top_btn(ui, Btn::Raster(glyphs::LYRICS), "lyrics", if want { "Hide lyrics" } else { "Lyrics" }, false, want) {
        app.mini.toggle_lyrics();
    }

    // The pin, alone in the strip's top-left corner and on a layer above it: the whole strip
    // is a drag area that moves the window, and a press it takes never reaches the button.
    // A layer of its own takes the press first, so clicking the pin only pins.
    let pinned = app.mini.pinned;
    let at = r.min + vec2(6.0, 6.0);
    let mut pin_clicked = false;
    egui::Area::new(scope.with("mini-pin")).order(egui::Order::Middle).fixed_pos(at).show(&ctx, |ui| {
        let back = Rect::from_min_max(at - Vec2::splat(4.0), at + vec2(32.0, 32.0));
        ui.painter().rect_filled(back, 9.0, pill.gamma_multiply(reveal));
        let b = Rect::from_min_size(at, Vec2::splat(28.0));
        let resp = ui.interact(b, btn_id("pin"), Sense::click());
        let name = if pinned { "Stop keeping on top" } else { "Keep on top" };
        let resp = named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand);
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.12);
        if h > 0.0 {
            ui.painter().rect_filled(b, 6.0, theme::glass(0.10 * h).gamma_multiply(reveal));
        }
        let fg = theme::lerp_color(theme::text_dim(), theme::text(), if pinned { 1.0 } else { h });
        // The glyph is what the click does: pinned, it offers to come off the top.
        icon_at(ui, b.center(), if pinned { icon::PIN_OFF } else { icon::PIN }, 14.0, fg.gamma_multiply(reveal));
        if resp.has_focus() {
            ui.painter().rect_stroke(b.shrink(1.0), 6.0, egui::Stroke::new(1.5, theme::text()), egui::StrokeKind::Inside);
        }
        pin_clicked = resp.clicked() && (reveal > 0.5 || resp.has_focus() || resp.hovered());
    });
    if pin_clicked {
        app.mini.set_pinned(&ctx, !pinned);
    }

    // One group, centred under the text: like, previous, play, next, volume. Hovering
    // the volume slides a volume track over the group, as the player bar's does.
    let cy = art.bottom() - 17.0;
    let cx = (x0 + x1) / 2.0;
    let enabled = track.is_some();
    let vol = app.audio.volume();
    let vpos = pos2(cx + 88.0, cy);
    let vhit = Rect::from_center_size(vpos, Vec2::splat(32.0));
    let slider = Rect::from_min_max(pos2(cx - 88.0, cy - 10.0), pos2(cx + 58.0, cy + 10.0));
    let open_id = ui.scope_id().with("mini-vol-open");
    let was_open = ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
    // Pointer position, not `rect_contains_pointer`: layers above the mini player's own
    // (window chrome, the drag regions) would otherwise hide the pointer from it.
    let hover = |r: Rect| ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| r.contains(p));
    let vol_band = Rect::from_min_max(pos2(slider.left() - 4.0, cy - 14.0), pos2(vhit.right(), cy + 14.0));
    let open = hover(vhit) || ui.rect_contains_pointer(vhit) || (was_open && (hover(vol_band) || ui.input(|i| i.pointer.any_down())));
    ui.data_mut(|d| d.insert_temp(open_id, open));
    if open != was_open {
        ctx.request_repaint();
    }
    let reveal = theme::anim_bool(&ctx, "mini-vol-reveal", open, 0.15);
    if reveal < 0.5 {
        // The same glyphs the player bar draws, a size down.
        let small = |ui: &mut Ui, c: Pos2, key: &str, glyph: &'static [u8], name: &str| -> bool {
            let b = Rect::from_center_size(c, Vec2::splat(34.0));
            let resp = ui.interact(b, ui.scope_id().with(("mini-t", key)), Sense::click());
            let resp = named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand);
            let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);
            glyphs::draw(ui, c, 20.0, glyph, theme::lerp_color(theme::text_dim(), theme::text(), h));
            resp.clicked()
        };
        if small(ui, pos2(cx - 44.0, cy), "prev", glyphs::PREVIOUS, "Previous") && enabled {
            app.prev();
        }
        if small(ui, pos2(cx + 44.0, cy), "next", glyphs::NEXT, "Next") && enabled {
            app.next();
        }
        let playing = app.playing();
        let pr = Rect::from_center_size(pos2(cx, cy), Vec2::splat(40.0));
        let presp = named(ui.interact(pr, ui.scope_id().with("mini-play"), Sense::click()), if playing { "Pause" } else { "Play" }).on_hover_cursor(egui::CursorIcon::PointingHand);
        let ph = theme::anim_bool(&ctx, presp.id.with("h"), presp.hovered(), 0.15);
        let c = pr.center();
        let morph = theme::anim_bool(&ctx, "mini-play-morph", playing || app.buffering, 0.18);
        let fg = theme::lerp_color(theme::text_dim(), theme::text(), 0.55 + 0.45 * ph);
        if app.buffering {
            let a = now as f32 * 6.0;
            let pts = (0..20).map(|i| c + vec2((a + i as f32 * 0.22).cos(), (a + i as f32 * 0.22).sin()) * 7.0).collect();
            ui.painter().add(egui::Shape::Path(egui::epaint::PathShape::line(pts, egui::epaint::PathStroke::new(2.0, fg))));
            ctx.request_repaint_after(Duration::from_millis(33));
        } else {
            if morph < 1.0 {
                glyphs::draw(ui, c, 26.0, glyphs::PLAY, theme::with_alpha(fg, 1.0 - morph));
            }
            if morph > 0.0 {
                glyphs::draw(ui, c, 26.0, glyphs::PAUSE, theme::with_alpha(fg, morph));
            }
        }
        if presp.clicked() && enabled {
            app.toggle();
        }

        if let Some(t) = &track {
            // What's heard, like Now playing's menu: a song version standing in for the music video.
            let t = if hold { t.clone() } else { app.playing_track().cloned().unwrap_or_else(|| t.clone()) };
            let liked = app.liked.contains(&t.id);
            let lpos = pos2(cx - 88.0, cy);
            let lresp = ui.interact(Rect::from_center_size(lpos, Vec2::splat(32.0)), ui.scope_id().with("mini-like"), Sense::click());
            let name = if liked { "Remove like" } else { "Like" };
            let lresp = named(lresp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand);
            let lh = theme::anim_bool(&ctx, lresp.id.with("h"), lresp.hovered(), 0.15);
            let pop = theme::anim_bool(&ctx, ("liked", &t.id), liked, 0.3);
            // A liked song keeps a soft disc behind its star, as the player bar's does.
            let disc = (0.17 * pop + 0.10 * lh).min(0.30);
            if disc > 0.0 {
                ui.painter().circle_filled(lpos, 16.0, theme::glass(disc));
            }
            let size = 18.0 * (1.0 + 0.25 * (pop * std::f32::consts::PI).sin());
            let tint = theme::lerp_color(theme::lerp_color(theme::text_dim(), theme::text(), lh), app.accent_color(), pop);
            glyphs::draw(ui, lpos, size, glyphs::STAR, tint);
            if pop > 0.0 {
                glyphs::draw(ui, lpos, size, glyphs::STAR_FILLED, theme::with_alpha(app.accent_color(), pop));
            }
            if lresp.clicked() {
                app.actions.push(Action::Like(t.clone(), !liked));
            }
        }
    } else {
        let track_r = Rect::from_center_size(slider.center(), vec2(slider.width(), 4.0));
        ui.painter().rect_filled(track_r, 2.0, theme::glass(0.16));
        let fill = Rect::from_min_size(track_r.min, vec2(track_r.width() * vol, 4.0));
        ui.painter().rect_filled(fill, 2.0, theme::text());
        ui.painter().circle_filled(pos2(fill.right(), track_r.center().y), 6.0, theme::text());
        let sresp = ui.interact(slider.expand2(vec2(0.0, 8.0)), ui.scope_id().with("mini-vol-slider"), Sense::click_and_drag());
        let sresp = sresp.on_hover_cursor(egui::CursorIcon::PointingHand);
        sresp.widget_info(|| egui::WidgetInfo::slider(true, vol as f64, "Volume"));
        if let Some(p) = sresp.interact_pointer_pos()
            && (sresp.clicked() || sresp.dragged())
        {
            app.audio.set_volume(((p.x - track_r.left()) / track_r.width()).clamp(0.0, 1.0));
        }
    }

    // Volume: hover slides the track out, click mutes, scroll to change.
    let vol = app.audio.volume();
    let vresp = ui.interact(vhit, ui.scope_id().with("mini-vol"), Sense::click());
    let pct = format!("Volume {}%", (vol * 100.0).round());
    let vresp = named(vresp, "Mute").on_hover_text(pct).on_hover_cursor(egui::CursorIcon::PointingHand);
    let vh = theme::anim_bool(&ctx, vresp.id.with("h"), vresp.hovered(), 0.15);
    ui.painter().circle_filled(vpos, 16.0, theme::glass(0.1 * vh));
    let vglyph = glyphs::VOLUME[glyphs::volume_step(vol)];
    glyphs::draw(ui, vpos, 18.0, vglyph, theme::lerp_color(theme::text_dim(), theme::text(), vh));
    if vresp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            app.audio.set_volume((vol + scroll / 600.0).clamp(0.0, 1.0));
        }
    }
    if vresp.clicked() {
        super::shell::toggle_mute(app, &ctx);
    }

    // The line being sung, over the strip: a hairline between them and the line centred
    // in its own band.
    if side {
        ui.painter().line_segment([pos2(col.left() + pad, band.bottom()), pos2(col.right() - pad, band.bottom())], egui::Stroke::new(1.0, theme::glass(0.16)));
        lyric_band(app, ui, band, now);
    }

    seek_line(app, ui, col, track.as_ref());
}

/// The line the singer is on: the last one that has started, and where in it they are. The
/// same audible position the full player's lyrics follow — what the device plays lags the
/// player by its buffer, and a handoff is still the outgoing song's tail.
fn cue(app: &mut App, now: f64) -> Option<Cue> {
    let (id, set) = app.lyrics.take()?;
    let cue = timed(app, &id, &set, now);
    app.lyrics = Some((id.clone(), set));
    cue.map(|(index, shot, t)| Cue { id, index, shot, t })
}

pub(super) fn desktop_notch(app: &mut App, ctx: &egui::Context) {
    const CANVAS_W: f32 = 640.0;
    const CANVAS_H: f32 = 200.0;

    let expanded_key = egui::Id::unique("whis-notch-expanded");
    let tucked_key = egui::Id::unique("whis-notch-tucked");
    let peek_hover_key = egui::Id::unique("whis-notch-peek-hover");
    let compact_w_id = egui::Id::unique("whis-notch-compact-w");
    let curve_id = egui::Id::unique("whis-notch-curve");
    let lyric_anim_id = egui::Id::unique("whis-notch-lyric-anim");

    let tucked = ctx.data(|d| d.get_temp::<bool>(tucked_key)).unwrap_or(false);
    let expanded = !tucked && ctx.data(|d| d.get_temp::<bool>(expanded_key)).unwrap_or(false);
    let peek_hovered = ctx.data(|d| d.get_temp::<bool>(peek_hover_key)).unwrap_or(false);

    let now = ctx.input(|i| i.time);
    let track = app.playing_track().cloned().or_else(|| app.current().cloned());
    let active_cue = cue(app, now);
    let lyric = active_cue.as_ref().map(|c| c.shot.text.clone());
    let compact_text = lyric.clone().unwrap_or_else(|| match &track {
        Some(tr) => tr.title.clone(),
        None => "WhisMuno".to_owned(),
    });

    // Track lyric line transitions for smooth rising animation.
    let (prev_text, cur_text, changed_at) = ctx.data(|d| {
        d.get_temp::<(String, String, f64)>(lyric_anim_id)
            .unwrap_or_else(|| (String::new(), compact_text.clone(), now - 1.0))
    });
    let (prev_text, cur_text, changed_at) = if cur_text != compact_text {
        let next = (cur_text, compact_text.clone(), now);
        ctx.data_mut(|d| d.insert_temp(lyric_anim_id, next.clone()));
        next
    } else {
        (prev_text, cur_text, changed_at)
    };
    let rise_q = ((now - changed_at) / 0.36).clamp(0.0, 1.0) as f32;
    let lyric_animating = rise_q < 1.0;

    // Quantize to 16px steps with 20px shrink hysteresis so short words don't pump the pill.
    let measured_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(compact_text.clone(), theme::semibold(13.5), Color32::WHITE).size().x
    });
    let quantized_w = ((measured_w / 16.0).ceil() * 16.0 + 132.0).clamp(292.0, 596.0);
    let prev_compact_w = ctx.data(|d| d.get_temp::<f32>(compact_w_id)).unwrap_or(quantized_w);
    let compact_w = if quantized_w < prev_compact_w && (prev_compact_w - quantized_w) < 20.0 {
        prev_compact_w
    } else {
        quantized_w
    };
    ctx.data_mut(|d| d.insert_temp(compact_w_id, compact_w));

    let (target_w, target_h) = if tucked {
        if peek_hovered { (194.0, 26.0) } else { (182.0, 24.0) }
    } else if expanded {
        (468.0, 190.0)
    } else {
        (compact_w, 40.0)
    };

    // Smooth 0.30s cubic S-curve transition (zero velocity at start & end, zero jitter).
    let (mut from_w, mut to_w, mut from_h, mut to_h, mut start_time) = ctx.data(|d| {
        d.get_temp::<(f32, f32, f32, f32, f64)>(curve_id)
            .unwrap_or((target_w, target_w, target_h, target_h, now - 1.0))
    });
    let u_prev = ((now - start_time) / 0.30).clamp(0.0, 1.0) as f32;
    let s_prev = u_prev * u_prev * (3.0 - 2.0 * u_prev);
    let cur_w_now = from_w + (to_w - from_w) * s_prev;
    let cur_h_now = from_h + (to_h - from_h) * s_prev;
    if (target_w - to_w).abs() > 1.0 || (target_h - to_h).abs() > 1.0 {
        from_w = cur_w_now;
        from_h = cur_h_now;
        to_w = target_w;
        to_h = target_h;
        start_time = now;
        ctx.data_mut(|d| d.insert_temp(curve_id, (from_w, to_w, from_h, to_h, start_time)));
    }
    let u = ((now - start_time) / 0.30).clamp(0.0, 1.0) as f32;
    let s = u * u * (3.0 - 2.0 * u);
    let cur_w = from_w + (to_w - from_w) * s;
    let cur_h = from_h + (to_h - from_h) * s;
    let animating = u < 1.0;

    let size = vec2(CANVAS_W, CANVAS_H);
    let scale = ctx.pixels_per_point();
    let pos = crate::fullscreen::work_area().map(|area| {
        pos2((area.center().x / scale - CANVAS_W / 2.0).round(), (area.top() / scale).round())
    });

    let mut builder = egui::ViewportBuilder::default()
        .with_title("Whis Notch")
        .with_inner_size(size)
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_taskbar(false)
        .with_active(false)
        .with_always_on_top();
    if let Some(p) = pos {
        builder = builder.with_position(p);
    }

    let id = egui::ViewportId::from_hash_of("whis-desktop-notch");
    ctx.show_viewport_immediate(id, builder, |ui, _| {
        if ui.input(|i| i.viewport().close_requested()) {
            app.desktop_notch = false;
            return;
        }

        ui.visuals_mut().override_text_color = Some(Color32::from_rgb(242, 248, 252));
        let bounds = ui.max_rect();
        let pill = Rect::from_min_size(
            pos2(bounds.center().x - cur_w / 2.0, bounds.top()),
            vec2(cur_w, cur_h),
        );
        let mint = Color32::from_rgb(102, 235, 203);
        let cyan = Color32::from_rgb(56, 189, 248);
        let bg_top = Color32::from_rgb(5, 8, 14);
        let bg_bot = Color32::from_rgb(10, 17, 28);
        let rim = if tucked {
            Color32::from_rgba_unmultiplied(102, 235, 203, if peek_hovered { 155 } else { 85 })
        } else if expanded {
            Color32::from_rgba_unmultiplied(102, 235, 203, 72)
        } else {
            Color32::from_rgba_unmultiplied(102, 235, 203, 52)
        };
        let bottom_r: u8 = if tucked { 13 } else if expanded { 28 } else { 20 };
        let radius = egui::CornerRadius { nw: 0, ne: 0, sw: bottom_r, se: bottom_r };

        clip_notch_region(ui.ctx(), pill, bounds, animating);
        ui.painter().rect_filled(pill, radius, bg_top);
        // Subtle ambient gradient glow at the bottom of the Dynamic Island
        let glow_band = Rect::from_min_max(pos2(pill.left(), pill.top() + pill.height() * 0.35), pill.max);
        ui.painter().rect_filled(glow_band, radius, bg_bot.gamma_multiply(0.55));
        // Double glass rim: outer mint-tinted edge + inner hairline highlight
        ui.painter().rect_stroke(pill, radius, egui::Stroke::new(1.0, rim), egui::StrokeKind::Inside);
        ui.painter().rect_stroke(
            pill.shrink(1.2),
            egui::CornerRadius { nw: 0, ne: 0, sw: bottom_r.saturating_sub(1), se: bottom_r.saturating_sub(1) },
            egui::Stroke::new(0.6, Color32::from_rgba_unmultiplied(255, 255, 255, 16)),
            egui::StrokeKind::Inside,
        );

        let playing = app.playing();
        let draw_eq = |ui: &Ui, center: Pos2, bars: usize, max_h: f32| {
            let mid = (bars as f32 - 1.0) / 2.0;
            for i in 0..bars {
                let phase = now * 8.6 + (i as f64) * 1.18;
                let wave = (phase.sin() * 0.5 + 0.5) as f32;
                let h = if playing { 3.5 + max_h * wave } else { 3.2 };
                let x = center.x + (i as f32 - mid) * 3.8;
                let bar = Rect::from_center_size(pos2(x, center.y), vec2(2.2, h));
                let col = theme::lerp_color(cyan, mint, wave);
                ui.painter().rect_filled(bar, 1.1, col);
            }
        };

        let draw_notch_lyric = |p: &egui::Painter, rect: Rect, base_size: f32, align_center: bool| {
            let e = ease(rise_q);
            if !prev_text.is_empty() && rise_q < 1.0 {
                let out_sz = base_size - 0.9 * e;
                let out_alpha = ((1.0 - e) * (1.0 - e) * 210.0) as u8;
                let out_c = Color32::from_rgba_unmultiplied(195, 215, 228, out_alpha);
                let g_out = p.layout_no_wrap(prev_text.clone(), theme::semibold(out_sz), out_c);
                let out_x = if align_center { rect.center().x - g_out.size().x / 2.0 } else { rect.left() };
                p.galley(pos2(out_x, rect.center().y - g_out.size().y / 2.0 - e * 14.0), g_out, out_c);
            }

            let dy_in = (1.0 - e) * 14.0;
            let sz = base_size - 0.9 * (1.0 - e);
            if let Some(c) = &active_cue
                && c.shot.text == cur_text
            {
                let sung = sung_chars(&c.shot, c.t);
                let dim_c = Color32::from_rgba_unmultiplied(142, 160, 178, (e * 215.0).round() as u8);
                let g_dim = p.layout_no_wrap(cur_text.clone(), theme::semibold(sz), dim_c);
                let text_x = if align_center { rect.center().x - g_dim.size().x / 2.0 } else { rect.left() };
                let text_pos = pos2(text_x, rect.center().y - g_dim.size().y / 2.0 + dy_in);
                p.galley(text_pos, g_dim.clone(), dim_c);
                let (_, edge_x) = lit_edge(&g_dim, sung);

                // Soft aqua bloom ahead of the active syllable
                let bloom_c = Color32::from_rgba_unmultiplied(56, 189, 248, (e * 175.0).round() as u8);
                let g_bloom = p.layout_no_wrap(cur_text.clone(), theme::semibold(sz), bloom_c);
                let bloom_clip = Rect::from_min_max(
                    rect.min,
                    pos2((text_pos.x + edge_x + 22.0).clamp(rect.left(), rect.right()), rect.bottom()),
                );
                p.with_clip_rect(bloom_clip).galley(text_pos - vec2(0.0, 0.5), g_bloom, bloom_c);

                // Electric mint karaoke wave
                let mint_c = Color32::from_rgba_unmultiplied(102, 235, 203, (e * 250.0).round() as u8);
                let g_mint = p.layout_no_wrap(cur_text.clone(), theme::semibold(sz), mint_c);
                let mint_clip = Rect::from_min_max(
                    rect.min,
                    pos2((text_pos.x + edge_x + 12.0).clamp(rect.left(), rect.right()), rect.bottom()),
                );
                p.with_clip_rect(mint_clip).galley(text_pos - vec2(0.0, 0.8), g_mint, mint_c);

                // Crisp white sung core
                let lit_c = Color32::from_rgba_unmultiplied(255, 255, 255, (e * 255.0).round() as u8);
                let g_lit = p.layout_no_wrap(cur_text.clone(), theme::semibold(sz), lit_c);
                let lit_clip = Rect::from_min_max(
                    rect.min,
                    pos2((text_pos.x + edge_x + 1.5).clamp(rect.left(), rect.right()), rect.bottom()),
                );
                p.with_clip_rect(lit_clip).galley(text_pos - vec2(0.0, 1.0), g_lit, lit_c);
            } else {
                let in_c = Color32::from_rgba_unmultiplied(255, 255, 255, (e * 255.0).round() as u8);
                let g_in = p.layout_no_wrap(cur_text.clone(), theme::semibold(sz), in_c);
                let in_x = if align_center { rect.center().x - g_in.size().x / 2.0 } else { rect.left() };
                p.galley(pos2(in_x, rect.center().y - g_in.size().y / 2.0 + dy_in), g_in, in_c);
            }
        };

        let mut tucked_now = tucked;
        let mut expanded_now = expanded;

        ui.scope_builder(egui::UiBuilder::new().max_rect(pill), |ui| {
            if tucked {
                let hit = ui.interact(pill, ui.id().with("notch-peek"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Show WhisMuno Notch");
                ctx.data_mut(|d| d.insert_temp(peek_hover_key, hit.hovered()));

                let eq_pos = pos2(pill.left() + 18.0, pill.center().y);
                ui.painter().circle_filled(eq_pos, 8.0, Color32::from_rgb(16, 46, 44));
                ui.painter().circle_stroke(eq_pos, 8.0, egui::Stroke::new(0.8, mint.gamma_multiply(0.55)));
                icon_at(ui, eq_pos, icon::MUSIC, 10.0, mint);
                let chev_pos = pos2(pill.right() - 16.0, pill.center().y);
                icon_at(ui, chev_pos, icon::CHEVRON_DOWN, 13.0, mint);

                let label_rect = Rect::from_min_max(pos2(eq_pos.x + 10.0, pill.top()), pos2(chev_pos.x - 10.0, pill.bottom()));
                let peek_label = track.as_ref().map(|tr| tr.title.as_str()).unwrap_or("WhisMuno");
                ui.painter().with_clip_rect(label_rect).text(
                    label_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    peek_label,
                    theme::semibold(11.5),
                    Color32::from_rgb(232, 246, 242),
                );
                if hit.clicked() {
                    tucked_now = false;
                    expanded_now = false;
                }
                return;
            }

            if !expanded {
                // Micro playback progress laser line along bottom edge of compact notch
                let pos_s = app.audio.status.position_smooth().as_secs_f32();
                let dur_s = app.audio.status.duration().map(|d| d.as_secs_f32())
                    .or_else(|| track.as_ref().and_then(|tr| tr.duration).map(|d| d as f32))
                    .unwrap_or(0.0);
                if dur_s > 0.0 {
                    let frac = (pos_s / dur_s).clamp(0.0, 1.0);
                    let line_w = (pill.width() - 36.0).max(20.0);
                    let bar_bg = Rect::from_min_size(pos2(pill.left() + 18.0, pill.bottom() - 2.4), vec2(line_w, 1.5));
                    ui.painter().rect_filled(bar_bg, 0.75, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
                    let bar_fg = Rect::from_min_size(bar_bg.min, vec2(line_w * frac, 1.5));
                    ui.painter().rect_filled(bar_fg, 0.75, mint.gamma_multiply(0.85));
                }

                egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 6)).show(ui, |ui| {
                    let row = ui.available_rect_before_wrap();
                    let thumb = Rect::from_center_size(pos2(row.left() + 13.0, row.center().y), vec2(24.0, 24.0));
                    match &track {
                        Some(tr) => paint_cover(ui, thumb, &tr.thumbs, 7.5, 1.0),
                        None => {
                            ui.painter().rect_filled(thumb, 7.5, Color32::from_rgb(16, 46, 44));
                            icon_at(ui, thumb.center(), icon::MUSIC, 14.0, mint);
                        }
                    }
                    if playing {
                        ui.painter().rect_stroke(thumb, 7.5, egui::Stroke::new(1.1, mint.gamma_multiply(0.75)), egui::StrokeKind::Outside);
                    }

                    let tuck_rect = Rect::from_center_size(pos2(row.right() - 11.0, row.center().y), vec2(22.0, 22.0));
                    let tuck_hit = ui.interact(tuck_rect, ui.id().with("notch-tuck-btn"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Hide to mini pill");
                    if tuck_hit.hovered() {
                        ui.painter().rect_filled(tuck_rect, 11.0, Color32::from_rgba_unmultiplied(102, 235, 203, 32));
                    }
                    icon_at(
                        ui,
                        tuck_rect.center(),
                        icon::CHEVRON_UP,
                        13.5,
                        if tuck_hit.hovered() { mint } else { Color32::from_rgb(186, 204, 218) },
                    );
                    if tuck_hit.clicked() {
                        tucked_now = true;
                        expanded_now = false;
                    }

                    let eq_center = pos2(tuck_rect.left() - 16.0, row.center().y);
                    draw_eq(ui, eq_center, 4, 9.0);

                    let content = Rect::from_min_max(
                        pos2(thumb.right() + 10.0, row.top()),
                        pos2(eq_center.x - 14.0, row.bottom()),
                    );
                    draw_notch_lyric(&ui.painter().with_clip_rect(content), content, 13.5, true);

                    let body_rect = Rect::from_min_max(pill.min, pos2(tuck_rect.left() - 2.0, pill.bottom()));
                    let hit = ui.interact(body_rect, ui.id().with("notch-expand"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(&compact_text);
                    if hit.clicked() {
                        expanded_now = true;
                    } else if hit.secondary_clicked() {
                        tucked_now = true;
                    }
                });
                return;
            }

            // Expanded Dynamic Island Media Card (WhisMuno Pro Deck)
            egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let art_size = 64.0;
                    let (art_rect, _) = ui.allocate_exact_size(vec2(art_size, art_size), Sense::hover());
                    match &track {
                        Some(tr) => paint_cover(ui, art_rect, &tr.thumbs, 13.0, 1.0),
                        None => {
                            ui.painter().rect_filled(art_rect, 13.0, Color32::from_rgba_unmultiplied(102, 235, 203, 18));
                            icon_at(ui, art_rect.center(), icon::MUSIC, 24.0, mint);
                        }
                    }
                    ui.painter().rect_stroke(
                        art_rect,
                        13.0,
                        egui::Stroke::new(1.0, if playing { mint.gamma_multiply(0.55) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 24) }),
                        egui::StrokeKind::Inside,
                    );
                    ui.add_space(12.0);
                    let info_w = (ui.available_width() - 96.0).max(60.0);
                    ui.allocate_ui(vec2(info_w, art_size), |ui| {
                        ui.vertical(|ui| {
                            // Top micro-badge: WHISMUNO LIVE + Equalizer
                            ui.horizontal(|ui| {
                                let (dot_r, _) = ui.allocate_exact_size(vec2(6.0, 6.0), Sense::hover());
                                ui.painter().circle_filled(dot_r.center(), 3.0, if playing { mint } else { Color32::from_rgb(120, 136, 152) });
                                ui.add_space(3.0);
                                ui.label(
                                    egui::RichText::new(if lyric.is_some() { "WHISMUNO · LIVE LYRICS" } else { "WHISMUNO ISLAND" })
                                        .font(theme::semibold(9.5))
                                        .color(mint.gamma_multiply(0.9)),
                                );
                            });
                            ui.add_space(2.0);
                            let (title, artist) = match &track {
                                Some(tr) => (tr.title.as_str(), tr.artist_line()),
                                None => ("Nothing playing", "Play a song in WhisMuno".to_owned()),
                            };
                            text_at(ui, ui.cursor().min, title, theme::semibold(14.5), Color32::WHITE, info_w);
                            ui.add_space(17.0);
                            if !artist.is_empty() {
                                text_at(ui, ui.cursor().min, &artist, theme::regular(11.8), Color32::from_rgb(156, 174, 194), info_w);
                                ui.add_space(14.0);
                            }
                            if lyric.is_some() {
                                let (lyr_r, _) = ui.allocate_exact_size(vec2(info_w, 17.0), Sense::hover());
                                draw_notch_lyric(&ui.painter().with_clip_rect(lyr_r), lyr_r, 12.2, false);
                            }
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        let top_btn = |ui: &mut Ui, glyph: char, sz: f32, tip: &str, danger: bool| {
                            let (r, hit) = ui.allocate_exact_size(vec2(25.0, 25.0), Sense::click());
                            let fill = if hit.hovered() {
                                if danger {
                                    Color32::from_rgba_unmultiplied(239, 68, 68, 70)
                                } else {
                                    Color32::from_rgba_unmultiplied(102, 235, 203, 38)
                                }
                            } else {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 13)
                            };
                            ui.painter().rect_filled(r, 12.5, fill);
                            let fg = if hit.hovered() {
                                if danger { Color32::from_rgb(254, 202, 202) } else { mint }
                            } else {
                                Color32::from_rgb(203, 213, 225)
                            };
                            icon_at(ui, r.center(), glyph, sz, fg);
                            hit.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked()
                        };
                        if top_btn(ui, icon::X, 13.5, "Turn off notch", true) {
                            app.desktop_notch = false;
                        }
                        ui.add_space(4.0);
                        if top_btn(ui, icon::SHRINK, 13.0, "Collapse to compact lyrics", false) {
                            expanded_now = false;
                        }
                        ui.add_space(4.0);
                        if top_btn(ui, icon::CHEVRON_UP, 13.5, "Hide to mini pill", false) {
                            tucked_now = true;
                            expanded_now = false;
                        }
                    });
                });

                // Interactive seekable progress bar + timestamps
                ui.add_space(9.0);
                let pos_s = app.audio.status.position_smooth().as_secs_f32();
                let dur_s = app.audio.status.duration().map(|d| d.as_secs_f32())
                    .or_else(|| track.as_ref().and_then(|tr| tr.duration).map(|d| d as f32))
                    .unwrap_or(0.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(fmt_time(pos_s as u32)).size(10.5).color(Color32::from_rgb(160, 178, 198)));
                    let bar_w = (ui.available_width() - 52.0).max(40.0);
                    let (hit_rect, seek_hit) = ui.allocate_exact_size(vec2(bar_w, 14.0), Sense::click_and_drag());
                    let thick = if seek_hit.hovered() || seek_hit.dragged() { 5.2 } else { 4.0 };
                    let prog_rect = Rect::from_center_size(hit_rect.center(), vec2(bar_w, thick));
                    ui.painter().rect_filled(prog_rect, thick / 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24));
                    if dur_s > 0.0 {
                        let frac = (pos_s / dur_s).clamp(0.0, 1.0);
                        let fill = Rect::from_min_size(prog_rect.min, vec2(prog_rect.width() * frac, prog_rect.height()));
                        ui.painter().rect_filled(fill, thick / 2.0, mint);
                        if seek_hit.hovered() || seek_hit.dragged() {
                            ui.painter().circle_filled(pos2(fill.right(), prog_rect.center().y), 5.2, Color32::WHITE);
                        }
                        if (seek_hit.clicked() || seek_hit.drag_stopped())
                            && let Some(p) = seek_hit.interact_pointer_pos()
                        {
                            let target_frac = ((p.x - prog_rect.left()) / prog_rect.width().max(1.0)).clamp(0.0, 1.0);
                            app.actions.push(Action::Seek(target_frac * dur_s));
                        }
                    }
                    ui.label(egui::RichText::new(fmt_time(dur_s as u32)).size(10.5).color(Color32::from_rgb(160, 178, 198)));
                });

                // Modern vector transport controls + Like + Side Lyrics toggle + Volume + Live EQ
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    let control = |ui: &mut Ui, label: &str, glyph: char, sz: f32, accent: bool, active: bool| {
                        let box_sz = if accent { vec2(40.0, 30.0) } else { vec2(30.0, 30.0) };
                        let (r, hit) = ui.allocate_exact_size(box_sz, Sense::click());
                        if accent {
                            ui.painter().rect_filled(r, 15.0, if hit.hovered() { mint } else { mint.gamma_multiply(0.92) });
                            icon_at(ui, r.center(), glyph, sz, Color32::from_rgb(6, 12, 16));
                        } else {
                            let fill = if active {
                                Color32::from_rgba_unmultiplied(102, 235, 203, 36)
                            } else if hit.hovered() {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 26)
                            } else {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 11)
                            };
                            ui.painter().rect_filled(r, 12.0, fill);
                            icon_at(ui, r.center(), glyph, sz, if active { mint } else { Color32::WHITE });
                        }
                        hit.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(label).clicked()
                    };

                    if let Some(tr) = &track {
                        let liked = app.liked.contains(&tr.id);
                        if control(ui, if liked { "Liked" } else { "Like song" }, icon::HEART, 14.0, false, liked) {
                            app.actions.push(Action::Like(tr.clone(), !liked));
                        }
                        ui.add_space(4.0);
                    }
                    if control(ui, "Previous", icon::SKIP_BACK, 14.5, false, false) { app.prev(); }
                    ui.add_space(4.0);
                    if control(ui, if playing { "Pause" } else { "Play" }, if playing { icon::PAUSE } else { icon::PLAY }, 16.0, true, false) { app.toggle(); }
                    ui.add_space(4.0);
                    if control(ui, "Next", icon::SKIP_FORWARD, 14.5, false, false) { app.next(); }
                    ui.add_space(6.0);
                    if control(ui, "Toggle Side Lyrics in App", icon::LIST_MUSIC, 14.0, false, app.lyrics_drawer) {
                        app.lyrics_drawer = !app.lyrics_drawer;
                        if app.lyrics_drawer {
                            app.queue_open = false;
                            app.np_open = false;
                        }
                    }
                    ui.add_space(8.0);

                    let vol = app.audio.volume();
                    let vol_glyph = if vol <= 0.001 { icon::VOLUME_X } else if vol < 0.5 { icon::VOLUME_1 } else { icon::VOLUME_2 };
                    let (mute_r, mute_hit) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                    if mute_hit.hovered() {
                        ui.painter().rect_filled(mute_r, 13.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20));
                    }
                    icon_at(ui, mute_r.center(), vol_glyph, 14.5, if mute_hit.hovered() { Color32::WHITE } else { Color32::from_rgb(180, 196, 210) });
                    if mute_hit.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Mute / Unmute").clicked() {
                        super::shell::toggle_mute(app, ui.ctx());
                    }

                    ui.add_space(4.0);
                    let (vol_hit_r, vol_hit) = ui.allocate_exact_size(vec2(84.0, 16.0), Sense::click_and_drag());
                    let vol_track = Rect::from_center_size(vol_hit_r.center(), vec2(84.0, 4.0));
                    ui.painter().rect_filled(vol_track, 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24));
                    let vol_fill = Rect::from_min_size(vol_track.min, vec2(vol_track.width() * vol.clamp(0.0, 1.0), 4.0));
                    ui.painter().rect_filled(vol_fill, 2.0, mint);
                    if vol_hit.hovered() || vol_hit.dragged() {
                        ui.painter().circle_filled(pos2(vol_fill.right(), vol_track.center().y), 4.5, Color32::WHITE);
                    }
                    if (vol_hit.clicked() || vol_hit.dragged())
                        && let Some(p) = vol_hit.interact_pointer_pos()
                    {
                        app.audio.set_volume(((p.x - vol_track.left()) / vol_track.width()).clamp(0.0, 1.0));
                    }
                    let _ = vol_hit.on_hover_cursor(egui::CursorIcon::PointingHand);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (eq_r, _) = ui.allocate_exact_size(vec2(22.0, 20.0), Sense::hover());
                        draw_eq(ui, eq_r.center(), 4, 8.5);
                    });
                });
            });
        });

        if tucked_now != tucked {
            ctx.data_mut(|d| d.insert_temp(tucked_key, tucked_now));
        }
        if expanded_now != expanded {
            ctx.data_mut(|d| d.insert_temp(expanded_key, expanded_now));
        }
        if tucked_now != tucked || expanded_now != expanded {
            ctx.request_repaint();
        }
        if (!tucked && playing) || animating || lyric_animating {
            let ms = if animating || lyric_animating { 16 } else { 50 };
            ui.ctx().request_repaint_after(Duration::from_millis(ms));
        }
    });
}

#[cfg(windows)]
fn clip_notch_region(ctx: &egui::Context, pill: Rect, canvas: Rect, animating: bool) {
    use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmSetWindowAttribute};
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, SetWindowRgn};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::w;

    // SAFETY: finds our own notch viewport by its title and sets DWM borderless + hit region.
    let Ok(hwnd) = (unsafe { FindWindowW(None, w!("Whis Notch")) }) else { return };
    if hwnd.is_invalid() {
        return;
    }

    let dwm_key = egui::Id::unique("whis-notch-dwm-hwnd");
    let hwnd_raw = hwnd.0 as isize;
    if ctx.data(|d| d.get_temp::<isize>(dwm_key)) != Some(hwnd_raw) {
        let border_none: u32 = 0xFFFFFFFE;
        let corner_none: u32 = 1;
        unsafe {
            let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(34), &border_none as *const u32 as *const _, size_of::<u32>() as u32);
            let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(33), &corner_none as *const u32 as *const _, size_of::<u32>() as u32);
        }
        ctx.data_mut(|d| d.insert_temp(dwm_key, hwnd_raw));
    }

    let ppp = ctx.pixels_per_point();
    let state = if animating {
        (0, 0, (canvas.width() * ppp).round() as i32, (canvas.height() * ppp).round() as i32)
    } else {
        (
            (pill.left() * ppp).round() as i32 - 2,
            0,
            (pill.right() * ppp).round() as i32 + 2,
            (pill.bottom() * ppp).round() as i32 + 2,
        )
    };

    let rgn_key = egui::Id::unique("whis-notch-rgn");
    if ctx.data(|d| d.get_temp::<(i32, i32, i32, i32)>(rgn_key)) == Some(state) {
        return;
    }
    let region = unsafe { CreateRectRgn(state.0, state.1, state.2, state.3) };
    if !region.is_invalid() {
        unsafe {
            let _ = SetWindowRgn(hwnd, Some(region), false);
        }
        ctx.data_mut(|d| d.insert_temp(rgn_key, state));
    }
}

#[cfg(not(windows))]
fn clip_notch_region(_: &egui::Context, _: Rect, _: Rect, _: bool) {}

fn timed(app: &mut App, id: &str, set: &crate::lyrics::LyricsSet, now: f64) -> Option<(usize, Shot, f64)> {
    use crate::lyrics::Sync;
    let p = app.lyric_prefs.choose(id, set)?;
    let lyrics = set.get(p)?;
    if lyrics.sync == Sync::None {
        return None;
    }
    let rich = lyrics.sync == Sync::Word;
    let playing = if app.mix_hold().is_some() { app.audio.status.outgoing_playing() } else { app.playing() };
    let pos = match app.mix_hold() {
        Some(h) => app.mix_hold_pos(h) as f64 - app.audio.status.latency().as_secs_f64(),
        None => app.audio.status.position_smooth().as_secs_f64() - app.audio.status.latency().as_secs_f64(),
    };
    let pos = app.notch_clock.tick(pos, playing, now);
    let t = 1000.0 * (pos - app.lyric_prefs.offset(id, p, rich));
    // A gap keeps the line before it on screen rather than blanking the band.
    let (index, l) = lyrics.lines.iter().enumerate().filter(|(_, l)| !l.instrumental && !l.text.trim().is_empty() && l.start as f64 <= t).next_back()?;
    Some((index, shot(l, rich), t))
}

/// The line's words with their times, counted in characters from its start — which is what
/// a swept highlight needs. Parts that don't add up to the text can't place one, and a
/// source that only times lines gets the whole line at once.
fn shot(l: &crate::lyrics::Line, words: bool) -> Shot {
    let chars = l.text.chars().count();
    let mut out = Vec::new();
    if words {
        let mut at = 0usize;
        for p in &l.parts {
            let n = p.text.chars().count();
            if !p.text.trim().is_empty() {
                out.push((at, n, p.start as f64, p.dur as f64));
            }
            at += n;
        }
        if at != chars {
            out.clear();
        }
    }
    if out.is_empty() {
        let dur = (l.dur as f64).clamp(1400.0, 4800.0) * 0.88;
        out.push((0, chars, l.start as f64, dur));
    }
    Shot { text: l.text.clone(), words: out }
}

/// The band's line, centred and in the lyrics font the player's settings choose: dim, with
/// the words already sung lit up to the singer, rising in as the line before it rises out.
fn lyric_band(app: &mut App, ui: &mut Ui, band: Rect, now: f64) {
    let style = &app.look().lyrics;
    match cue(app, now) {
        Some(c) => {
            let b = &mut app.mini.band;
            if b.id != c.id || b.cur.is_none() {
                // A song of its own, or the band's first line: it arrives, nothing leaves.
                b.id = c.id;
                b.index = c.index;
                b.cur = Some(c.shot);
                b.out = None;
                b.at = now;
            } else if c.index != b.index {
                // The line on its way out keeps the highlight the singing had reached: it
                // would otherwise dim the moment its successor starts rising in.
                let lit_to = b.cur.as_ref().map(|s| sung_chars(s, b.t)).unwrap_or_default();
                b.out = b.cur.take().map(|s| (s, now, lit_to));
                b.index = c.index;
                b.cur = Some(c.shot);
                b.at = now;
            }
            b.t = c.t;
        }
        None => {
            let b = &mut app.mini.band;
            b.id.clear();
            b.cur = None;
            b.out = None;
        }
    }
    let b = &app.mini.band;
    let lit = crate::preset::paint(&style.sung_color, app.accent_color());
    let size = (30.0 * style.size.clamp(0.6, 1.6)).min(band.width() / 13.0).max(15.0);
    let Some(cur) = &b.cur else {
        // Nothing to time: say the lyric is still coming only when it actually is.
        if app.lyrics.is_some() {
            return;
        }
        let font = theme::lyric_font_for(&style.font, style.weight, 17.0, "Loading lyrics…");
        let color = theme::text_faint();
        let g = widgets::galley_wrapped(ui, "Loading lyrics…", font, color, band.width() - 32.0, 1);
        ui.painter().galley(band.center() - g.size() / 2.0, g, color);
        return;
    };
    let rise = (size * 0.8).min(band.height() * 0.35);
    let mut moving = false;
    if let Some((out, since, lit_to)) = &b.out {
        let q = ((now - since) / RISE).clamp(0.0, 1.0) as f32;
        moving |= q < 1.0;
        draw_shot(ui, band, out, style, size, lit, Some(*lit_to), -ease(q) * rise, 1.0 - q);
    }
    let e = ((now - b.at) / RISE).clamp(0.0, 1.0) as f32;
    moving |= e < 1.0;
    draw_shot(ui, band, cur, style, size, lit, Some(sung_chars(cur, b.t)), (1.0 - ease(e)) * rise, e);
    if moving {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

/// One line of the band at `dy` below its place, faded to `a`: the whole line dim and, up to
/// character `lit_to`, the same line lit. A row's worth of clip each, so a wrapped line
/// lights the rows the singing has left behind whole.
fn draw_shot(ui: &Ui, band: Rect, sh: &Shot, style: &crate::preset::LyricStyle, size: f32, lit: Color32, lit_to: Option<f32>, dy: f32, a: f32) {
    if a <= 0.01 {
        return;
    }
    let width = band.width() - 32.0;
    let font = theme::lyric_font_for(&style.font, style.weight, size, &sh.text);
    let dim = theme::with_alpha(lit, 0.34 * a);
    let g = widgets::galley_wrapped(ui, &sh.text, font.clone(), dim, width, 2);
    let at = band.center() - g.size() / 2.0 + vec2(0.0, dy);
    // A line on its way in or out is partly outside the band; the strip under it isn't
    // the line's to write on.
    let p = ui.painter().with_clip_rect(band);
    p.galley(at, g.clone(), dim);
    let Some(chars) = lit_to else {
        return;
    };
    let color = theme::with_alpha(lit, a);
    let said = widgets::galley_wrapped(ui, &sh.text, font, color, width, 2);
    let (row, edge) = lit_edge(&g, chars);
    for (ri, r) in said.rows.iter().enumerate().take(row + 1) {
        let right = if ri == row { at.x + edge } else { band.right() };
        let top = at.y + r.pos.y;
        let clip = Rect::from_min_max(pos2(band.left(), top - 1.0), pos2(right + 1.0, top + r.row.size.y + 1.0));
        p.with_clip_rect(clip.intersect(band)).galley(at, said.clone(), color);
    }
}

/// How far into the line the singing is, in characters: through the words it has left
/// behind and partway into the one it is on.
fn sung_chars(sh: &Shot, t: f64) -> f32 {
    let mut done = 0.0f32;
    for &(i, n, start, dur) in &sh.words {
        let e = if dur <= 0.0 {
            if t >= start { 1.0 } else { 0.0 }
        } else {
            ((t - start) / dur).clamp(0.0, 1.0) as f32
        };
        if e < 1.0 {
            return i as f32 + e * n as f32;
        }
        done = (i + n) as f32;
    }
    done
}

/// The galley's row that character `ci` falls on, and the x within the galley where it is.
fn lit_edge(g: &egui::Galley, ci: f32) -> (usize, f32) {
    let mut before = 0usize;
    for (ri, r) in g.rows.iter().enumerate() {
        let n = r.glyphs.len();
        let k = (ci - before as f32).clamp(0.0, n as f32);
        if k < n as f32 || ri + 1 == g.rows.len() {
            let i = k as usize;
            let x = match (n == 0, r.glyphs.get(i)) {
                (true, _) => r.pos.x,
                (_, Some(gl)) => {
                    let x0 = r.pos.x + gl.pos.x;
                    let x1 = r.glyphs.get(i + 1).map_or(x0 + gl.advance_width, |next| r.pos.x + next.pos.x);
                    x0 + (x1 - x0) * (k - i as f32)
                }
                (_, None) => r.pos.x + r.size.x,
            };
            return (ri, x);
        }
        before += n;
    }
    (0, 0.0)
}

/// Ease-out: the quick part first, settling into place.
fn ease(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}


/// A line along the bottom; thicker on hover, with the time under the pointer.
fn seek_line(app: &mut App, ui: &mut Ui, r: Rect, track: Option<&crate::model::Track>) {
    let status = app.audio.status.clone();
    let hold = app.mix_hold().map(|h| (h.total, app.mix_hold_pos(h)));
    let total = match &hold {
        Some((total, _)) => *total,
        None => status.duration().map(|d| d.as_secs_f32()).or(track.and_then(|t| t.duration).map(|d| d as f32)).unwrap_or(0.0),
    };
    let smooth = match &hold {
        Some((_, pos)) => *pos,
        None => status.position_smooth().as_secs_f32(),
    };
    let held = hold.is_some();
    let pos = app.seek_drag.unwrap_or(smooth).min(total.max(0.0));
    // Runs the window's full width, like Apple's strip; the corner radius is small
    // enough that the bottom edge rounds it off on its own.
    let (l, rr) = (r.left(), r.right());
    let hit = Rect::from_min_max(pos2(l, r.bottom() - 10.0), pos2(rr, r.bottom()));
    let resp = ui.interact(hit, ui.scope_id().with("mini-seek"), if held { Sense::hover() } else { Sense::click_and_drag() });
    let resp = if total > 0.0 && !held { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    resp.widget_info(|| egui::WidgetInfo::slider(total > 0.0 && !held, pos as f64, "Seek"));
    let h = theme::anim_bool(ui.ctx(), "mini-seek-h", total > 0.0 && !held && (resp.hovered() || resp.dragged()), 0.15);
    let thick = 4.0 + 2.0 * h;
    let line = Rect::from_min_max(pos2(l, r.bottom() - thick), pos2(rr, r.bottom()));
    ui.painter().rect_filled(line, 0.0, theme::glass(0.2));
    let frac = if total > 0.0 { pos / total } else { 0.0 };
    ui.painter().rect_filled(Rect::from_min_size(line.min, vec2(line.width() * frac, thick)), 0.0, app.accent_color());
    let at = |p: Pos2| ((p.x - l) / (rr - l).max(1.0)).clamp(0.0, 1.0) * total;
    if h > 0.0
        && let Some(p) = resp.hover_pos().or(resp.interact_pointer_pos())
    {
        let txt = format!("{} / {}", fmt_time(at(p) as u32), fmt_time(total as u32));
        let g = widgets::galley(ui, &txt, theme::semibold(11.5), Color32::WHITE, 120.0);
        let half = g.size().x / 2.0 + 6.0;
        let x = p.x.clamp(r.left() + half + 2.0, r.right() - half - 2.0);
        let bg = Rect::from_center_size(pos2(x, line.top() - 13.0), g.size() + vec2(12.0, 5.0));
        ui.painter().rect_filled(bg, 5.0, theme::shade(0.8).gamma_multiply(h));
        ui.painter().galley(bg.min + vec2(6.0, 2.5), g, Color32::WHITE.gamma_multiply(h));
    }
    if total > 0.0
        && let Some(p) = resp.interact_pointer_pos()
    {
        if resp.dragged() {
            app.seek_drag = Some(at(p));
        }
        if resp.drag_stopped() || resp.clicked() {
            app.seek_drag = None;
            app.actions.push(Action::Seek(at(p)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_place_is_the_work_areas_corner() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(2560.0, 1400.0));
        let p = corner_place(area, Vec2::ZERO, 1.0, 1.0);
        assert_eq!((p.x, p.y, p.w, p.h), (2560.0 - 400.0 - 16.0, 1400.0 - 112.0 - 16.0, SIZE.x, SIZE.y));
        // Scaled: pixels grow with the scale, points don't.
        let p = corner_place(area, vec2(0.0, 0.0), 1.5, 1.0);
        assert_eq!((p.x, p.y, p.w), (2560.0 - 600.0 - 24.0, 1400.0 - 168.0 - 24.0, SIZE.x));
        // A second monitor left of the main one has negative coordinates.
        let left = Rect::from_min_max(pos2(-1920.0, 100.0), pos2(0.0, 1140.0));
        let p = corner_place(left, vec2(2.0, 2.0), 1.0, 1.0);
        assert_eq!((p.x, p.y), (-402.0 - 16.0, 1140.0 - 114.0 - 16.0));
        // Never past the area's top left, however small it is.
        let tiny = Rect::from_min_max(pos2(10.0, 20.0), pos2(200.0, 50.0));
        let p = corner_place(tiny, Vec2::ZERO, 1.0, 1.0);
        assert_eq!((p.x, p.y), (10.0, 20.0));
        // Zoomed in, a point is bigger: the strip still ends at the margin.
        let p = corner_place(area, Vec2::ZERO, 1.0, 2.0);
        assert_eq!((p.x, p.y, p.zoom), (2560.0 - 800.0 - 16.0, 1400.0 - 224.0 - 16.0, 2.0));
    }

    #[test]
    fn a_lost_full_window_is_centred() {
        let area = Rect::from_min_max(pos2(1920.0, 0.0), pos2(3840.0, 1040.0));
        let p = centered(area, vec2(1200.0, 780.0), Vec2::ZERO, 1.0, 1.0);
        assert_eq!((p.x, p.y, p.w, p.h), (1920.0 + 360.0, 130.0, 1200.0, 780.0));
        // Bigger than the area: its top left stays on it.
        let p = centered(area, vec2(1000.0, 800.0), Vec2::ZERO, 2.0, 1.0);
        assert_eq!((p.x, p.y), (1920.0, 0.0));
    }

    #[test]
    fn remembered_sizes_stay_in_limits() {
        let p = Place { x: 5.0, y: 6.0, w: 2000.0, h: 300.0, scale: 1.25, zoom: 1.0 };
        assert_eq!(clamp_size(p, false), Place { w: MAX_W, h: SIZE.y, ..p });
        assert_eq!(clamp_size(Place { w: 10.0, ..p }, false).w, MIN_W);
        // With lyrics the window is the strip plus the band over it, and no wider.
        assert_eq!(clamp_size(Place { w: 10.0, ..p }, true), Place { w: MIN_W, h: SIZE.y + LYR_H, ..p });
        assert_eq!(clamp_size(p, true).w, MAX_W);
    }

    #[test]
    fn full_window_is_saved_in_eframes_format() {
        let p = Place { x: 814.0, y: -183.0, w: 1200.0, h: 812.0, scale: 1.0, zoom: 1.0 };
        assert_eq!(
            p.eframe_window(false),
            "(normal_geometry:(outer_position_pixels:Some((x:814.0,y:-183.0)),scale_factor:1.0,inner_size_points:(x:1200.0,y:812.0)),fullscreen:false,maximized:false)"
        );
        let s = p.eframe_window(true);
        assert!(s.ends_with("maximized:true)"));
    }

    #[test]
    fn places_round_trip_through_settings() {
        let p = Place { x: -1500.0, y: 900.0, w: 420.0, h: SIZE.y, scale: 1.5, zoom: 1.25 };
        let back: Place = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back, p);
        // Saved before places had a zoom: unzoomed.
        let old: Place = serde_json::from_str(r#"{"x":1.0,"y":2.0,"w":400.0,"h":112.0,"scale":1.0}"#).unwrap();
        assert_eq!(old.zoom, 1.0);
        // A full window kept at 100% comes back the same size in pixels at 125%.
        assert_eq!(Place { w: 1250.0, h: 1000.0, zoom: 1.0, ..p }.size_at(1.25), vec2(1000.0, 800.0));
        let m = Mini::load(|k| (k == "mini_window").then(|| serde_json::to_string(&p).unwrap()));
        assert_eq!(m.place, Some(p));
        assert_eq!(m.pinned, PINNED_BY_DEFAULT);
        let m = Mini::load(|k| (k == "mini_pinned").then(|| "0".to_owned()));
        assert!(!m.pinned && m.place.is_none() && !m.active());
    }

    #[test]
    fn the_band_lights_where_the_singing_is() {
        let line = |start: i32, text: &str, parts: &[(i32, i32, &str)]| crate::lyrics::Line {
            start,
            dur: 4000,
            text: text.to_owned(),
            parts: parts.iter().map(|(s, d, t)| crate::lyrics::Part { start: *s, dur: *d, text: t.to_string(), bg: false }).collect(),
            ..Default::default()
        };
        // Word times counted in characters from the line's start; spaces count, and light
        // nothing of their own.
        let sh = shot(&line(1000, "ah bu", &[(1000, 500, "ah"), (1500, 100, " "), (2000, 1000, "bu")]), true);
        assert_eq!(sh.words, vec![(0, 2, 1000.0, 500.0), (3, 2, 2000.0, 1000.0)]);
        assert_eq!(sung_chars(&sh, 1250.0), 1.0);
        assert_eq!(sung_chars(&sh, 2500.0), 4.0);
        assert_eq!(sung_chars(&sh, 9000.0), 5.0);
        // Parts that don't add up to the text, and a source that only times lines, light
        // the line whole when it starts.
        for (words, l) in [(true, line(1000, "ah bu", &[(1000, 500, "ah")])), (false, line(1000, "ah bu", &[]))] {
            let sh = shot(&l, words);
            assert_eq!(sh.words, vec![(0, 5, 1000.0, 0.0)]);
            assert_eq!((sung_chars(&sh, 999.0), sung_chars(&sh, 1000.0)), (0.0, 5.0));
        }
    }
}
