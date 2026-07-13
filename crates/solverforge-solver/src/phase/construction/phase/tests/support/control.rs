#[derive(Clone, Debug)]
struct BlockingPoint {
    state: Arc<(Mutex<BlockingPointState>, Condvar)>,
}

#[derive(Debug)]
struct BlockingPointState {
    blocked: bool,
    released: bool,
}

impl BlockingPoint {
    fn new() -> Self {
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

    fn block(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.blocked = true;
        condvar.notify_all();
        while !state.released {
            state = condvar.wait(state).unwrap();
        }
    }

    fn wait_until_blocked(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        while !state.blocked {
            state = condvar.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.released = true;
        condvar.notify_all();
    }
}

#[derive(Clone, Debug)]
struct BlockingEvaluationGate {
    block_at: usize,
    seen: Arc<AtomicUsize>,
    blocker: BlockingPoint,
    delay: Duration,
}

impl BlockingEvaluationGate {
    fn new(block_at: usize) -> Self {
        Self {
            block_at,
            seen: Arc::new(AtomicUsize::new(0)),
            blocker: BlockingPoint::new(),
            delay: Duration::ZERO,
        }
    }

    fn delaying(delay: Duration) -> Self {
        Self {
            block_at: usize::MAX,
            seen: Arc::new(AtomicUsize::new(0)),
            blocker: BlockingPoint::new(),
            delay,
        }
    }

    fn on_evaluation(&self) {
        if !self.delay.is_zero() {
            std::thread::park_timeout(self.delay);
        }
        let seen = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
        if seen == self.block_at {
            self.blocker.block();
        }
    }

    fn wait_until_blocked(&self) {
        self.blocker.wait_until_blocked();
    }

    fn release(&self) {
        self.blocker.release();
    }
}
