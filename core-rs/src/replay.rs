const MIN_TICK_MS: i64 = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub frames: Vec<Vec<u8>>,
    pub wait_ms: u64,
    pub percent: f64,
    pub log_time_s: u64,
    pub at_end: bool,
}

#[derive(Debug, Clone)]
pub struct Replay {
    entries: Vec<(u64, Vec<u8>)>,
    index: usize,
    speed: f64,
    start_log_us: u64,
    duration_us: u64,
    current_us: u64,
    playback_start_ms: u64,
    playback_start_log_us: u64,
    playing: bool,
}

impl Replay {
    pub fn new(entries: Vec<(u64, Vec<u8>)>) -> Self {
        let start = entries.first().map(|e| e.0).unwrap_or(0);
        let last = entries.last().map(|e| e.0).unwrap_or(start);
        Replay { entries, index: 0, speed: 1.0, start_log_us: start, duration_us: last.saturating_sub(start), current_us: start, playback_start_ms: 0, playback_start_log_us: 0, playing: false }
    }

    pub fn from_tlog(bytes: &[u8]) -> Self {
        Self::new(crate::tlog::entries(bytes))
    }

    pub fn duration_s(&self) -> u64 {
        self.duration_us / 1_000_000
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn play(&mut self, now_ms: u64) {
        self.playing = true;
        self.set_speed(self.speed, now_ms);
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn set_speed(&mut self, speed: f64, now_ms: u64) {
        self.speed = speed;
        self.playback_start_ms = now_ms;
        self.playback_start_log_us = self.current_us;
    }

    pub fn percent(&self) -> f64 {
        match self.duration_us {
            0 => 0.0,
            d => (self.current_us.saturating_sub(self.start_log_us) as f64 / d as f64) * 100.0,
        }
    }

    pub fn seek(&mut self, percent: f64) -> f64 {
        self.playing = false;
        let desired = self.start_log_us + (percent.clamp(0.0, 100.0) / 100.0 * self.duration_us as f64) as u64;
        self.index = self.entries.iter().position(|(ts, _)| *ts >= desired).unwrap_or(self.entries.len().saturating_sub(1));
        self.current_us = self.entries.get(self.index).map(|e| e.0).unwrap_or(self.start_log_us);
        self.percent()
    }

    pub fn tick(&mut self, now_ms: u64) -> Batch {
        let mut frames = Vec::new();
        let mut wait = 0i64;
        while wait < MIN_TICK_MS {
            let Some((_, frame)) = self.entries.get(self.index) else {
                self.playing = false;
                return Batch { frames, wait_ms: 0, percent: self.percent(), log_time_s: self.current_us.saturating_sub(self.start_log_us) / 1_000_000, at_end: true };
            };
            frames.push(frame.clone());
            self.index += 1;
            let Some((next_us, _)) = self.entries.get(self.index) else {
                self.playing = false;
                return Batch { frames, wait_ms: 0, percent: 100.0, log_time_s: self.duration_us / 1_000_000, at_end: true };
            };
            self.current_us = *next_us;
            let movement_ms = ((self.current_us.saturating_sub(self.playback_start_log_us) / 1000) as f64 / self.speed) as u64;
            wait = (self.playback_start_ms + movement_ms) as i64 - now_ms as i64;
        }
        Batch { frames, wait_ms: wait as u64, percent: self.percent(), log_time_s: self.current_us.saturating_sub(self.start_log_us) / 1_000_000, at_end: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(step_us: u64, count: usize) -> Vec<(u64, Vec<u8>)> {
        (0..count).map(|i| (1_000_000 + i as u64 * step_us, vec![i as u8; 3])).collect()
    }

    #[test]
    fn frames_closer_than_three_milliseconds_go_out_together_and_the_wait_scales_with_speed() {
        let mut replay = Replay::new(log(1_000, 20));
        replay.play(10_000);
        let batch = replay.tick(10_000);
        assert_eq!(batch.frames.len(), 3);
        assert_eq!(batch.wait_ms, 3);
        assert!(!batch.at_end);
        let mut fast = Replay::new(log(1_000, 20));
        fast.play(10_000);
        fast.set_speed(2.0, 10_000);
        assert_eq!(fast.tick(10_000).frames.len(), 6);
    }

    #[test]
    fn playback_reaches_the_end_and_reports_percent_and_time() {
        let mut replay = Replay::new(log(2_000_000, 4));
        assert_eq!(replay.duration_s(), 6);
        replay.play(0);
        let first = replay.tick(0);
        assert_eq!((first.frames.len(), first.wait_ms, first.log_time_s), (1, 2000, 2));
        assert!((first.percent - 33.333).abs() < 0.01);
        let second = replay.tick(2000);
        let third = replay.tick(4000);
        let last = replay.tick(6000);
        assert_eq!((second.frames.len(), third.frames.len(), last.frames.len()), (1, 1, 1));
        assert!(last.at_end && !replay.is_playing() && last.percent == 100.0);
    }

    #[test]
    fn seeking_lands_on_the_first_frame_at_or_after_the_requested_time() {
        let mut replay = Replay::new(log(1_000_000, 11));
        let percent = replay.seek(50.0);
        assert_eq!(percent, 50.0);
        let batch = replay.tick(0);
        assert_eq!(batch.frames[0], vec![5u8; 3]);
        assert_eq!(replay.seek(150.0), 100.0);
        assert_eq!(replay.seek(-5.0), 0.0);
    }

    #[test]
    fn a_recorded_frame_reads_back_with_the_swap_rule_for_old_logs() {
        let recorded = crate::tlog::record(1_700_000_000_000_000, &[0xfe, 1, 2]);
        assert_eq!(recorded.len(), 11);
        assert_eq!(crate::tlog::parse_timestamp(recorded[..8].try_into().unwrap(), 1_800_000_000_000_000), 1_700_000_000_000_000);
        let little = 1_700_000_000_000_000u64.to_le_bytes();
        assert_eq!(crate::tlog::parse_timestamp(little, 1_800_000_000_000_000), 1_700_000_000_000_000);
        let sample = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        let replay = Replay::from_tlog(&sample);
        assert!(replay.duration_s() > 0 && replay.entries.len() > 1000);
        assert_eq!(replay.entries.len(), crate::tlog::parse(&sample).frames + crate::tlog::parse(&sample).undecodable);
    }
}
