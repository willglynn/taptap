use std::time::{Duration, SystemTime};
use crate::pv::link::InvalidSlotNumber;
use crate::pv::SlotCounter;

/// A data structure collating absolute timestamps to slot counters.
#[derive(Debug, Clone)]
pub struct SlotClock {
    // SystemTime timestamp per thousand ticks, i.e. per ±5s, wrapping with the counter
    times: [SystemTime; 48],
    last_index: usize,
    last_time: SystemTime,
}

const NOMINAL_DURATION_PER_SLOT: Duration = Duration::from_millis(5);
const NOMINAL_DURATION_PER_INDEX: Duration = Duration::from_millis(5 * 1000);

impl SlotClock {
    pub fn new(slot_counter: SlotCounter, time: SystemTime) -> Result<Self, InvalidSlotNumber> {
        let (index, offset) = Self::index_and_offset(slot_counter)?;
        let index_time = time - offset;

        let mut table = Self {
            times: [index_time; 48],
            last_index: index,
            last_time: time,
        };

        // Walk backwards, assuming nominal time for each
        let mut index_time = index_time;
        let mut i = index;
        loop {
            // Subtract one index
            i = (47 + i) % 48;
            if i == index {
                // Go around once
                break;
            }

            // Subtract one duration
            index_time -= NOMINAL_DURATION_PER_INDEX;
            // Assign
            table.times[i] = index_time;
        }

        Ok(table)
    }

    fn index_and_offset(slot_counter: SlotCounter) -> Result<(usize, Duration), InvalidSlotNumber> {
        slot_counter.slot_number().map(|n| {
            let absolute_slot = (slot_counter.epoch() as u8 as usize) * 12000 + (u16::from(n)) as usize;
            let index = absolute_slot / 1000;
            let offset = NOMINAL_DURATION_PER_SLOT * (absolute_slot % 1000) as u32;
            (index, offset)
        })
    }

    pub fn set(&mut self, slot_counter: SlotCounter, time: SystemTime) -> Result<(), InvalidSlotNumber> {
        let (index, offset) = Self::index_and_offset(slot_counter)?;

        if self.last_time > time {
            // Clock went backwards
            // Replace the table entirely
            log::warn!("time went backwards: {:?} => {:?}", self.last_time, time);
            *self = Self::new(slot_counter, time)?;
            return Ok(());
        } else if self.last_index != index {
            // Assign this index
            let index_time = time - offset;

            // Set the entry
            self.times[index] = index_time;

            // Walk backwards, assuming nominal time for each
            let mut index_time = index_time;
            let mut i = index;
            loop {
                // Subtract one index
                i = (47 + i) % 48;
                if i == self.last_index {
                    // Don't clobber the last assigned slot
                    break;
                }

                // Subtract one duration
                index_time -= NOMINAL_DURATION_PER_INDEX;
                // Assign
                self.times[i] = index_time;
            }
        } else {
            // Don't reassign this index
        }

        // Record this assignment
        self.last_index = index;
        self.last_time = time;

        Ok(())
    }

    pub fn get(&self, slot_counter: SlotCounter) -> Result<SystemTime, InvalidSlotNumber> {
        // TODO: interpolate for accuracy? Or don't, because measurements come in at thousands.
        let (index, offset) = Self::index_and_offset(slot_counter)?;
        Ok(self.times[index] + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        // Pick a time and call it "x"
        let x = SystemTime::UNIX_EPOCH + Duration::from_secs(1723500000);

        // Assume a gateway told us it's 0xc000
        let mut clock = SlotClock::new(SlotCounter::from(0xc000), x).unwrap();

        // 0x8000 was one minute ago
        assert_eq!(clock.get(SlotCounter::from(0x8000)), Ok(x - Duration::from_secs(60)));
        // 0x4000 was two minutes ago
        assert_eq!(clock.get(SlotCounter::from(0x4000)), Ok(x - Duration::from_secs(120)));
        // 0x0000 was three minutes ago
        assert_eq!(clock.get(SlotCounter::from(0x0000)), Ok(x - Duration::from_secs(180)));
        // 0xc000 + 1000 was three minutes 55 seconds ago
        assert_eq!(clock.get(SlotCounter::from(0xc000 + 1000)), Ok(x - Duration::from_secs(180 + 55)));

        // Advance to 0xc000 + 1000 at 5 seconds later
        let later = x + Duration::from_secs(5);
        clock.set(SlotCounter::from(0xc000 + 1000), later).unwrap();

        // 0x8000 was one minute before x
        assert_eq!(clock.get(SlotCounter::from(0x8000)), Ok(x - Duration::from_secs(60)));
        // 0x4000 was two minutes before x
        assert_eq!(clock.get(SlotCounter::from(0x4000)), Ok(x - Duration::from_secs(120)));
        // 0x0000 was three minutes before x
        assert_eq!(clock.get(SlotCounter::from(0x0000)), Ok(x - Duration::from_secs(180)));
        // 0xc000 + 1000 is x + 5
        assert_eq!(clock.get(SlotCounter::from(0xc000 + 1000)), Ok(x + Duration::from_secs(5)));
    }

    #[test]
    fn index_and_offset() {
        assert_eq!(SlotClock::index_and_offset(SlotCounter::ZERO), Ok((0, Duration::from_millis(0))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(999.into())), Ok((0, Duration::from_millis(999 * 5))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(1000.into())), Ok((1, Duration::from_millis(0))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(1999.into())), Ok((1, Duration::from_millis(999 * 5))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(2000.into())), Ok((2, Duration::from_millis(0))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(11999.into())), Ok((11, Duration::from_millis(999 * 5))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(12000.into())), Err(InvalidSlotNumber(12000)));
        assert_eq!(SlotClock::index_and_offset(SlotCounter(0x4000.into())), Ok((12, Duration::from_millis(0))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter((0x4000 + 999).into())), Ok((12, Duration::from_millis(999 * 5))));
        assert_eq!(SlotClock::index_and_offset(SlotCounter((0x4000 + 1000).into())), Ok((13, Duration::from_millis(0))));
    }
}