use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,

    // This state is only used for the tas controller interface
    Skipping,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TraceInterval {
    First(u32),
    Last(u32),
    Between(u32, u32),
}

impl TraceInterval {
    pub fn variant_name_simple(&self) -> &str {
        match self {
            TraceInterval::First(_) => "First",
            TraceInterval::Last(_) => "Last",
            TraceInterval::Between(_, _) => "Between",
        }
    }
}

impl Default for TraceInterval {
    fn default() -> Self {
        Self::Last(100)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct TraceDrawOptions {
    pub sphere_radius: f32,
    pub z_offset: f32,
    pub puzzle_click_indicator_distance_multiplier: f32,
    pub puzzle_click_indicator_radius: f32,
    pub interval: TraceInterval,
}

impl Default for TraceDrawOptions {
    fn default() -> Self {
        Self {
            sphere_radius: 0.05,
            z_offset: Default::default(),
            puzzle_click_indicator_distance_multiplier: Default::default(),
            puzzle_click_indicator_radius: 0.01,
            interval: Default::default(),
        }
    }
}

pub fn to_time(ticks: u32) -> String {
    let total_secs = ticks as f32 / 60.0;
    let mins = (total_secs / 60.0).floor() as u32;
    let secs = total_secs - (mins * 60) as f32;

    format!("{mins}m{secs:5.2}s")
}
