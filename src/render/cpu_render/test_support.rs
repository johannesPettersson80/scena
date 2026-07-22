use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct RayonBlockerState {
    started: usize,
    release: bool,
}

pub(super) struct RayonBlockers {
    state: Arc<(Mutex<RayonBlockerState>, Condvar)>,
}

impl Drop for RayonBlockers {
    fn drop(&mut self) {
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().expect("Rayon blocker state locks for release");
        state.release = true;
        wake.notify_all();
    }
}

pub(super) fn occupy_all_but_one_rayon_worker() -> RayonBlockers {
    let blocker_count = rayon::current_num_threads().saturating_sub(1);
    assert!(
        blocker_count > 0,
        "focused parallel-band regression requires at least two Rayon workers"
    );
    let state = Arc::new((Mutex::new(RayonBlockerState::default()), Condvar::new()));
    for _ in 0..blocker_count {
        let state = Arc::clone(&state);
        rayon::spawn(move || {
            let (lock, wake) = &*state;
            let mut guard = lock.lock().expect("Rayon blocker state locks");
            guard.started += 1;
            wake.notify_all();
            while !guard.release {
                guard = wake.wait(guard).expect("Rayon blocker wait resumes");
            }
        });
    }

    let deadline = Instant::now() + Duration::from_secs(10);
    let (lock, wake) = &*state;
    let mut guard = lock.lock().expect("Rayon blocker state locks for startup");
    while guard.started < blocker_count {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out occupying {blocker_count} Rayon workers; started {}",
            guard.started
        );
        let (next, timeout) = wake
            .wait_timeout(guard, remaining)
            .expect("Rayon blocker startup wait resumes");
        guard = next;
        assert!(
            !timeout.timed_out() || guard.started == blocker_count,
            "timed out occupying {blocker_count} Rayon workers; started {}",
            guard.started
        );
    }
    drop(guard);
    RayonBlockers { state }
}
