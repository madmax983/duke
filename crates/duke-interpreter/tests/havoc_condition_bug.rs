#![allow(missing_docs)]
// The ReentrantLock try_acquire logic used to drop the lock and return an exception if interrupted was true,
// but it failed to reacquire the lock before throwing InterruptedException, which violates the Java spec.
// We write a test to demonstrate the flaw in logic using simplified state logic.

#[derive(Debug)]
pub struct ReentrantLockState {
    pub owner: Option<usize>,
    pub hold_count: i32,
}

const fn reentrant_lock_try_acquire(state: &mut ReentrantLockState, thread_id: usize) -> bool {
    match state.owner {
        Some(owner) if owner != thread_id => false,
        Some(_) => {
            state.hold_count = state.hold_count.saturating_add(1);
            true
        }
        None => {
            state.owner = Some(thread_id);
            state.hold_count = 1;
            true
        }
    }
}

// Simulating condition_await_common's wake-up behavior
const fn simulated_condition_await_common(
    state: &mut ReentrantLockState,
    thread_id: usize,
    interrupted: bool,
) -> Result<(), &'static str> {
    // thread is woken up from sleep!

    // BAD BEHAVIOR (old code):
    // if interrupted { return Err("InterruptedException"); }
    // if reentrant_lock_try_acquire(state, thread_id) { return Ok(()); }

    // FIX BEHAVIOR (new code):
    if !reentrant_lock_try_acquire(state, thread_id) {
        return Ok(()); // retry
    }
    if interrupted {
        return Err("InterruptedException");
    }

    Ok(())
}

#[test]
fn test_condition_await_reacquires_lock_before_throwing() {
    let mut state = ReentrantLockState {
        owner: None,
        hold_count: 0,
    };
    let thread_id = 1;

    let result = simulated_condition_await_common(&mut state, thread_id, true);

    // We expect it to return the error
    assert_eq!(result, Err("InterruptedException"));

    // AND crucially, we expect the lock to be reacquired!
    assert_eq!(state.owner, Some(thread_id));
    assert_eq!(state.hold_count, 1);
}
