use mavlink::dialects::ardupilotmega::*;

use crate::mocklink::{AUTOPILOT, Out};
use crate::onboardlogs::LOG_DATA_LEN;

const LOG_ID: u16 = 0;
const LOG_SIZE: u32 = 1000;
const CHUNKS_PER_POLL: u32 = 12;

#[derive(Default)]
pub struct Logs {
    erased: bool,
    sending: Option<(u32, u32)>,
}

impl Logs {
    pub fn listed(&self) -> Out {
        let count = u16::from(!self.erased);
        (AUTOPILOT, MavMessage::LOG_ENTRY(LOG_ENTRY_DATA { id: LOG_ID, num_logs: count, last_log_num: count, time_utc: 0, size: if self.erased { 0 } else { LOG_SIZE } }))
    }

    pub fn requested(&mut self, request: &LOG_REQUEST_DATA_DATA) {
        let served = !self.erased && request.id == LOG_ID && request.ofs < LOG_SIZE;
        self.sending = served.then(|| (request.ofs, request.count.min(LOG_SIZE - request.ofs))).or(self.sending);
    }

    pub fn erase(&mut self) {
        self.erased = true;
        self.sending = None;
    }

    pub fn pump(&mut self) -> Vec<Out> {
        let Some((ofs, remaining)) = self.sending else { return Vec::new() };
        let end = ofs + remaining;
        let chunks: Vec<(u32, u32)> = (0..CHUNKS_PER_POLL).map(|i| ofs + i * LOG_DATA_LEN).take_while(|at| *at < end).map(|at| (at, LOG_DATA_LEN.min(end - at))).collect();
        let sent: u32 = chunks.iter().map(|(_, count)| count).sum();
        self.sending = (sent < remaining).then(|| (ofs + sent, remaining - sent));
        chunks.into_iter().map(|(at, count)| data(at, count)).collect()
    }
}

fn data(ofs: u32, count: u32) -> Out {
    let data = std::array::from_fn(|i| if (i as u32) < count { byte_at(ofs + i as u32) } else { 0 });
    (AUTOPILOT, MavMessage::LOG_DATA(LOG_DATA_DATA { ofs, id: LOG_ID, count: count as u8, data }))
}

fn byte_at(at: u32) -> u8 {
    (at % 251) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(ofs: u32, count: u32, id: u16) -> LOG_REQUEST_DATA_DATA {
        LOG_REQUEST_DATA_DATA { ofs, count, id, target_system: 1, target_component: 1 }
    }

    fn drained(logs: &mut Logs) -> Vec<LOG_DATA_DATA> {
        std::iter::from_fn(|| Some(logs.pump()).filter(|batch| !batch.is_empty()))
            .flatten()
            .filter_map(|(_, message)| match message {
                MavMessage::LOG_DATA(data) => Some(data),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_simulated_vehicle_lists_one_log_until_it_is_erased_like_qgc_mocklink() {
        let mut logs = Logs::default();
        let entry = |logs: &Logs| match logs.listed().1 {
            MavMessage::LOG_ENTRY(entry) => (entry.id, entry.num_logs, entry.size),
            _ => unreachable!(),
        };
        assert_eq!(entry(&logs), (0, 1, 1000));
        logs.erase();
        assert_eq!(entry(&logs), (0, 0, 0));
    }

    #[test]
    fn a_requested_range_streams_back_in_log_data_chunks_that_tile_it_exactly() {
        let mut logs = Logs::default();
        logs.requested(&request(0, 0xFFFF_FFFF, 0));
        let chunks = drained(&mut logs);
        let bytes: Vec<u8> = chunks.iter().flat_map(|c| c.data[..usize::from(c.count)].to_vec()).collect();
        assert_eq!(bytes.len(), 1000);
        assert_eq!(bytes, (0..1000).map(byte_at).collect::<Vec<u8>>());
        assert!(chunks.windows(2).all(|pair| pair[0].ofs + u32::from(pair[0].count) == pair[1].ofs));

        logs.requested(&request(950, 512, 0));
        assert_eq!(drained(&mut logs).iter().map(|c| u32::from(c.count)).sum::<u32>(), 50, "a request past the end is cut at the log's size");
    }

    #[test]
    fn requests_for_another_log_past_the_end_or_after_erase_send_nothing() {
        let mut logs = Logs::default();
        logs.requested(&request(0, 100, 3));
        logs.requested(&request(1000, 100, 0));
        assert!(drained(&mut logs).is_empty());
        logs.erase();
        logs.requested(&request(0, 100, 0));
        assert!(drained(&mut logs).is_empty());
    }
}
