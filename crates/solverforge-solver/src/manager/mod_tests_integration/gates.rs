use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Debug)]
pub(super) struct LifecycleStepGate {
    permit: Arc<(Mutex<bool>, Condvar)>,
}

impl LifecycleStepGate {
    pub(super) fn new_closed() -> Self {
        Self {
            permit: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub(super) fn allow_next_step(&self) {
        let (lock, condvar) = &*self.permit;
        let mut open = lock.lock().unwrap();
        *open = true;
        condvar.notify_all();
    }

    pub(super) fn wait_for_permit(&self) {
        let (lock, condvar) = &*self.permit;
        let mut open = lock.lock().unwrap();
        while !*open {
            open = condvar.wait(open).unwrap();
        }
        *open = false;
    }
}

#[derive(Clone, Debug)]
pub(super) struct BlockingPoint {
    state: Arc<(Mutex<BlockingPointState>, Condvar)>,
}

#[derive(Debug)]
struct BlockingPointState {
    blocked: bool,
    released: bool,
}

impl BlockingPoint {
    pub(super) fn new() -> Self {
        Self {
            state: Arc::new((
                Mutex::new(BlockingPointState {
                    blocked: false,
                    released: false,
                }),
                Condvar::new(),
            )),
        }
    }

    pub(super) fn block(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.blocked = true;
        condvar.notify_all();
        while !state.released {
            state = condvar.wait(state).unwrap();
        }
    }

    pub(super) fn wait_until_blocked(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        while !state.blocked {
            state = condvar.wait(state).unwrap();
        }
    }

    pub(super) fn release(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.released = true;
        condvar.notify_all();
    }
}

#[derive(Clone, Debug)]
pub(super) struct BlockingEvaluationGate {
    block_at: usize,
    seen: Arc<AtomicUsize>,
    blocker: BlockingPoint,
}

impl BlockingEvaluationGate {
    pub(super) fn new(block_at: usize) -> Self {
        Self {
            block_at,
            seen: Arc::new(AtomicUsize::new(0)),
            blocker: BlockingPoint::new(),
        }
    }

    pub(super) fn on_evaluation(&self) {
        let seen = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
        if seen == self.block_at {
            self.blocker.block();
        }
    }

    pub(super) fn wait_until_blocked(&self) {
        self.blocker.wait_until_blocked();
    }

    pub(super) fn release(&self) {
        self.blocker.release();
    }
}
