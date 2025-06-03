use wasm_bindgen::JsValue;
use web_sys::js_sys;
use web_time::Instant;

use crate::handle_new_perf_data;

pub struct PerformanceMetrics {
    total_frame_time: f64,
    number_of_frames: f64,
    highest_frame_time: f32,
    lowest_frame_time: f32,
    startup_time: f32,
    timer_since_last_frame: Instant,
    timer_since_last_reset: Instant,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_frame_time: 0.,
            highest_frame_time: -1.,
            lowest_frame_time: 100000.,
            startup_time: -1.,
            number_of_frames: 0.,
            timer_since_last_frame: Instant::now(),
            timer_since_last_reset: Instant::now(),
        }
    }

    pub fn time_new_frame(&mut self) {
        let frame_time = self.timer_since_last_frame.elapsed().as_secs_f64();
        self.total_frame_time += frame_time;
        self.number_of_frames += 1.;
        self.timer_since_last_frame = web_time::Instant::now();

        let frame_time = frame_time as f32;

        // Startup
        if self.startup_time < 0. {
            self.startup_time = frame_time;
            return;
        }

        if self.highest_frame_time < frame_time {
            self.highest_frame_time = frame_time;
        }

        if self.lowest_frame_time > frame_time {
            self.lowest_frame_time = frame_time;
        }
    }

    pub fn send_perf_event(&mut self) {
        if self.timer_since_last_reset.elapsed().as_secs_f64() < 0.2 {
            return;
        }

        let event_data = js_sys::Map::new();
        let js_str = |s: &str| JsValue::from_str(s);
        let js_f32 = |s: f32| JsValue::from_f64(s as f64);

        event_data.set(
            &js_str("avg_frame_time"),
            &js_f32(self.get_avg_frame_time()),
        );
        event_data.set(
            &js_str("highest_frame_time"),
            &js_f32(self.highest_frame_time),
        );
        event_data.set(
            &js_str("lowest_frame_time"),
            &js_f32(self.lowest_frame_time),
        );
        event_data.set(&js_str("startup_time"), &js_f32(self.startup_time));

        handle_new_perf_data(event_data);
        self.reset();
    }

    fn reset(&mut self) {
        self.total_frame_time = 0.;
        self.highest_frame_time = -1.;
        self.lowest_frame_time = 1000000.;
        self.number_of_frames = 0.;
        self.timer_since_last_frame = web_time::Instant::now();
        self.timer_since_last_reset = web_time::Instant::now();
    }

    fn get_avg_frame_time(&self) -> f32 {
        if self.number_of_frames == 0. {
            return 0.;
        }
        (self.total_frame_time / self.number_of_frames) as f32
    }
}
