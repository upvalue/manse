use eframe::egui;
use std::time::{Duration, Instant};

/// Performance tracking for debugging battery/CPU usage
#[derive(Default)]
pub struct PerfStats {
    /// When the current measurement window started
    window_start: Option<Instant>,
    /// Number of frames rendered in this window
    frame_count: u64,
    /// Number of PTY events processed
    pty_events: u64,
    /// Number of IPC requests processed
    ipc_requests: u64,
    /// Frames where scroll animation was active
    scroll_animation_frames: u64,
    /// Frames while minimized
    minimized_frames: u64,
    /// Frames with pointer (mouse) activity
    pointer_frames: u64,
    /// Frames with keyboard activity
    keyboard_frames: u64,
    /// Frames where window has focus
    focused_frames: u64,
}

impl PerfStats {
    pub fn on_frame(&mut self, ctx: &egui::Context) {
        self.frame_count += 1;

        ctx.input(|i| {
            if i.focused {
                self.focused_frames += 1;
            }
            if i.pointer.is_moving() || i.pointer.any_down() || i.pointer.any_released() {
                self.pointer_frames += 1;
            }
            if !i.keys_down.is_empty()
                || i
                    .events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)))
            {
                self.keyboard_frames += 1;
            }
        });
    }

    pub fn on_minimized(&mut self) {
        self.minimized_frames += 1;
    }

    pub fn on_scroll_anim(&mut self) {
        self.scroll_animation_frames += 1;
    }

    pub fn on_pty_event(&mut self) {
        self.pty_events += 1;
    }

    pub fn on_ipc_request(&mut self) {
        self.ipc_requests += 1;
    }

    /// Log performance stats if enabled and interval has elapsed
    pub fn maybe_log(&mut self, interval: f32) {
        if interval <= 0.0 {
            return;
        }

        let now = Instant::now();
        let window_start = self.window_start.get_or_insert(now);
        let elapsed = now.duration_since(*window_start);

        if elapsed >= Duration::from_secs_f32(interval) {
            let secs = elapsed.as_secs_f64();
            let fps = self.frame_count as f64 / secs;
            let s = &self;

            let explained = s.pty_events
                + s.scroll_animation_frames
                + s.minimized_frames
                + s.pointer_frames
                + s.keyboard_frames;
            let mystery = s.frame_count.saturating_sub(explained);

            log::info!(
                "[perf] {:.1}s: frames={} ({:.1} fps) | pty={} scroll={} pointer={} kbd={} focused={} | mystery={}",
                secs,
                s.frame_count,
                fps,
                s.pty_events,
                s.scroll_animation_frames,
                s.pointer_frames,
                s.keyboard_frames,
                s.focused_frames,
                mystery,
            );

            *self = PerfStats {
                window_start: Some(now),
                ..Default::default()
            };
        }
    }
}
