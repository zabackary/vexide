use alloc::collections::BTreeMap;
use core::task::Waker;

use vexide_core::time::Instant;

pub struct Sleepers {
    sleepers: BTreeMap<Instant, Waker>,
}

impl Sleepers {
    pub fn push(&mut self, waker: Waker, instant: Instant) {
        self.sleepers.insert(instant, waker);
    }

    pub fn pop(&mut self) -> Option<(Instant, Waker)> {
        self.sleepers.pop_first()
    }
}

pub struct Reactor {
    pub(crate) sleepers: Sleepers,
}

impl Reactor {
    pub const fn new() -> Self {
        Self {
            sleepers: Sleepers {
                sleepers: BTreeMap::new(),
            },
        }
    }

    pub fn tick(&mut self) {
        if let Some((time, sleeper)) = self.sleepers.pop() {
            if time >= Instant::now() {
                sleeper.wake();
            }
        }
    }
}
