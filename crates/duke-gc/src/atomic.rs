use crate::{OLD_BIT, patch_forwarded_slot};
use duke_runtime::Slot;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
#[derive(Debug)]
pub struct ReentrantLockState {
    /// Whether the Java constructor requested a fair lock. Duke currently
    /// records the bit for observability but schedules with the VM runtime.
    pub fair: bool,
    /// Host thread that owns the lock, if any.
    pub owner: Option<std::thread::ThreadId>,
    /// Reentrant hold count for the owner.
    pub hold_count: i32,
}

impl ReentrantLockState {
    /// Create an unlocked `ReentrantLock` state.
    #[must_use]
    pub const fn new(fair: bool) -> Self {
        Self {
            fair,
            owner: None,
            hold_count: 0,
        }
    }
}

/// One Duke Java thread waiting on a synthetic `Condition`.
#[derive(Debug)]
pub struct ConditionWaiter {
    /// Waiting host thread.
    pub thread_id: std::thread::ThreadId,
    /// Number of `ReentrantLock` holds to restore before `await` returns.
    pub released_hold_count: i32,
    /// True after `signal`, `signalAll`, or timeout.
    pub signaled: bool,
    /// Optional absolute timeout for `awaitNanos`.
    pub deadline: Option<std::time::Instant>,
    /// True when the wake-up came from timeout rather than signal.
    pub timed_out: bool,
}

/// Host-side state for a synthetic `Condition`.
#[derive(Debug)]
pub struct ConditionState {
    /// The `ReentrantLock` this condition is bound to.
    pub lock: Arc<Mutex<ReentrantLockState>>,
    /// Threads currently parked on this condition.
    pub waiters: Vec<ConditionWaiter>,
}

impl ConditionState {
    /// Create condition state bound to a `ReentrantLock`.
    #[must_use]
    pub const fn new(lock: Arc<Mutex<ReentrantLockState>>) -> Self {
        Self {
            lock,
            waiters: Vec::new(),
        }
    }
}

/// Which synthetic `ReentrantReadWriteLock` view a heap object represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadWriteLockViewKind {
    /// The read-lock view.
    Read,
    /// The write-lock view.
    Write,
}

/// Host-side state for synthetic `ReentrantReadWriteLock` objects.
#[derive(Debug, Default)]
pub struct ReadWriteLockState {
    /// Current writer, if any.
    pub writer: Option<std::thread::ThreadId>,
    /// Reentrant write holds for `writer`.
    pub write_hold_count: i32,
    /// Per-thread read hold counts.
    pub readers: HashMap<std::thread::ThreadId, i32>,
}

/// Callable shape for a synthetic executor task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorTaskKind {
    /// Invoke `Runnable.run()V`.
    Runnable,
    /// Invoke `Callable.call()Object`.
    Callable,
}

/// One queued synthetic executor task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutorTask {
    /// Heap reference to the `Future` object that receives task state.
    pub future_ref: u64,
    /// Heap reference to the submitted `Runnable` or `Callable`.
    pub task_ref: u64,
    /// Invocation shape.
    pub kind: ExecutorTaskKind,
}

/// Host-side state for synthetic `ExecutorService` instances.
#[derive(Debug)]
pub struct ExecutorState {
    /// Maximum number of worker threads this pool may create.
    pub max_workers: usize,
    /// Tasks not yet claimed by a worker.
    pub queue: VecDeque<ExecutorTask>,
    /// `shutdown()` has been called.
    pub shutdown: bool,
    /// Tasks currently executing.
    pub active: usize,
    /// Live host worker threads owned by this executor.
    pub workers: usize,
    /// Pool reached the JDK termination condition.
    pub terminated: bool,
}

impl ExecutorState {
    /// Create an empty executor state with at least one worker slot.
    #[must_use]
    pub fn new(max_workers: usize) -> Self {
        Self {
            max_workers: max_workers.max(1),
            queue: VecDeque::new(),
            shutdown: false,
            active: 0,
            workers: 0,
            terminated: false,
        }
    }

    /// Recompute termination after a queue, worker, or shutdown transition.
    pub fn refresh_terminated(&mut self) {
        self.terminated =
            self.shutdown && self.queue.is_empty() && self.active == 0 && self.workers == 0;
    }
}

/// Mutex/condvar pair for a synthetic executor.
#[derive(Debug)]
pub struct ExecutorShared {
    /// Mutable executor state.
    pub state: Mutex<ExecutorState>,
    /// Worker wake-up signal for new tasks or shutdown.
    pub available: Condvar,
}

impl ExecutorShared {
    /// Create shared executor state.
    #[must_use]
    pub fn new(max_workers: usize) -> Self {
        Self {
            state: Mutex::new(ExecutorState::new(max_workers)),
            available: Condvar::new(),
        }
    }
}

/// One Duke Java thread waiting for a synthetic `CountDownLatch`.
#[derive(Debug)]
pub struct CountDownLatchWaiter {
    /// Waiting host thread.
    pub thread_id: std::thread::ThreadId,
    /// Optional absolute timeout for timed await.
    pub deadline: Option<std::time::Instant>,
}

/// Host-side state for synthetic `CountDownLatch` instances.
#[derive(Debug)]
pub struct CountDownLatchState {
    /// Remaining count before the latch trips.
    pub count: i32,
    /// Threads currently waiting for the count to reach zero.
    pub waiters: Vec<CountDownLatchWaiter>,
}

impl CountDownLatchState {
    /// Create a latch with a non-negative initial count.
    #[must_use]
    pub const fn new(count: i32) -> Self {
        Self {
            count,
            waiters: Vec::new(),
        }
    }
}

/// One Duke Java thread waiting for synthetic `Semaphore` permits.
#[derive(Debug)]
pub struct SemaphoreWaiter {
    /// Waiting host thread.
    pub thread_id: std::thread::ThreadId,
    /// Number of permits requested.
    pub permits: i32,
    /// Optional absolute timeout for timed acquire.
    pub deadline: Option<std::time::Instant>,
}

/// Host-side state for synthetic `Semaphore` instances.
#[derive(Debug)]
pub struct SemaphoreState {
    /// Currently available permits. Java permits can grow without bound after
    /// unmatched release calls, so this is intentionally not tied to ownership.
    pub permits: i32,
    /// Whether constructor requested FIFO acquisition.
    pub fair: bool,
    /// FIFO queue used when fairness is enabled; also tracks retrying waiters.
    pub waiters: VecDeque<SemaphoreWaiter>,
}

impl SemaphoreState {
    /// Create a semaphore with the given permit count and fairness bit.
    #[must_use]
    pub const fn new(permits: i32, fair: bool) -> Self {
        Self {
            permits,
            fair,
            waiters: VecDeque::new(),
        }
    }
}

/// One Duke Java thread waiting at a synthetic `CyclicBarrier`.
#[derive(Debug)]
pub struct CyclicBarrierWaiter {
    /// Waiting host thread.
    pub thread_id: std::thread::ThreadId,
    /// Generation this waiter entered.
    pub generation: i32,
    /// Arrival index returned when the generation trips.
    pub arrival_index: i32,
    /// Optional absolute timeout for timed await.
    pub deadline: Option<std::time::Instant>,
    /// True once the waiter should receive `BrokenBarrierException`.
    pub broken: bool,
}

/// Host-side state for synthetic `CyclicBarrier` instances.
#[derive(Debug)]
pub struct CyclicBarrierState {
    /// Required parties per generation.
    pub parties: i32,
    /// Parties still needed in the current generation.
    pub count: i32,
    /// Monotonic generation number.
    pub generation: i32,
    /// True when the current generation is broken.
    pub broken: bool,
    /// Threads waiting in the current or just-tripped generation.
    pub waiters: Vec<CyclicBarrierWaiter>,
}

impl CyclicBarrierState {
    /// Create an unbroken barrier with all parties outstanding.
    #[must_use]
    pub const fn new(parties: i32) -> Self {
        Self {
            parties,
            count: parties,
            generation: 0,
            broken: false,
            waiters: Vec::new(),
        }
    }

    /// Start a fresh generation after a normal trip.
    pub const fn trip_generation(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.count = self.parties;
        self.broken = false;
    }

    /// Break the current generation and wake all current waiters.
    pub fn break_generation(&mut self) {
        for waiter in &mut self.waiters {
            if waiter.generation == self.generation {
                waiter.broken = true;
            }
        }
        self.generation = self.generation.saturating_add(1);
        self.count = self.parties;
        self.broken = true;
    }

    /// Reset to a new, unbroken generation.
    pub fn reset(&mut self) {
        self.break_generation();
        self.broken = false;
    }
}

/// Host-side payload backing synthetic `java.util.concurrent` objects.
#[derive(Debug)]
pub enum AtomicPayload {
    /// Backing cell for `AtomicInteger`.
    Int(Arc<AtomicI32>),
    /// Backing cell for `AtomicLong`.
    Long(Arc<AtomicI64>),
    /// Backing cell for `AtomicBoolean`.
    Bool(Arc<AtomicBool>),
    /// Backing cell for `AtomicReference`.
    Reference(Arc<Mutex<Slot>>),
    /// Coarse monitor for synthetic `ConcurrentHashMap` instances.
    ConcurrentMapLock(Arc<Mutex<()>>),
    /// Backing state for synthetic `ReentrantLock` instances.
    ReentrantLock(Arc<Mutex<ReentrantLockState>>),
    /// Backing state for synthetic `Condition` instances.
    Condition(Arc<Mutex<ConditionState>>),
    /// Shared state for a synthetic `ReentrantReadWriteLock` parent object.
    ReadWriteLock(Arc<Mutex<ReadWriteLockState>>),
    /// Read or write view object for a synthetic `ReentrantReadWriteLock`.
    ReadWriteLockView {
        /// Shared parent lock state.
        state: Arc<Mutex<ReadWriteLockState>>,
        /// View represented by the heap object.
        kind: ReadWriteLockViewKind,
    },
    /// Shared state for a synthetic `ExecutorService`.
    Executor(Arc<ExecutorShared>),
    /// Shared state for a synthetic `CountDownLatch`.
    CountDownLatch(Arc<Mutex<CountDownLatchState>>),
    /// Shared state for a synthetic `Semaphore`.
    Semaphore(Arc<Mutex<SemaphoreState>>),
    /// Shared state for a synthetic `CyclicBarrier`.
    CyclicBarrier(Arc<Mutex<CyclicBarrierState>>),
}

impl Clone for AtomicPayload {
    fn clone(&self) -> Self {
        match self {
            Self::Int(cell) => Self::Int(Arc::new(AtomicI32::new(cell.load(Ordering::SeqCst)))),
            Self::Long(cell) => Self::Long(Arc::new(AtomicI64::new(cell.load(Ordering::SeqCst)))),
            Self::Bool(cell) => Self::Bool(Arc::new(AtomicBool::new(cell.load(Ordering::SeqCst)))),
            Self::Reference(cell) => {
                let slot = *cell
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                Self::Reference(Arc::new(Mutex::new(slot)))
            }
            Self::ConcurrentMapLock(lock) => Self::ConcurrentMapLock(Arc::clone(lock)),
            Self::ReentrantLock(state) => Self::ReentrantLock(Arc::clone(state)),
            Self::Condition(state) => Self::Condition(Arc::clone(state)),
            Self::ReadWriteLock(state) => Self::ReadWriteLock(Arc::clone(state)),
            Self::ReadWriteLockView { state, kind } => Self::ReadWriteLockView {
                state: Arc::clone(state),
                kind: *kind,
            },
            Self::Executor(state) => Self::Executor(Arc::clone(state)),
            Self::CountDownLatch(state) => Self::CountDownLatch(Arc::clone(state)),
            Self::Semaphore(state) => Self::Semaphore(Arc::clone(state)),
            Self::CyclicBarrier(state) => Self::CyclicBarrier(Arc::clone(state)),
        }
    }
}

impl AtomicPayload {
    /// Create an `AtomicInteger` payload.
    #[must_use]
    pub fn int(value: i32) -> Self {
        Self::Int(Arc::new(AtomicI32::new(value)))
    }

    /// Create an `AtomicLong` payload.
    #[must_use]
    pub fn long(value: i64) -> Self {
        Self::Long(Arc::new(AtomicI64::new(value)))
    }

    /// Create an `AtomicBoolean` payload.
    #[must_use]
    pub fn bool(value: bool) -> Self {
        Self::Bool(Arc::new(AtomicBool::new(value)))
    }

    /// Create an `AtomicReference` payload.
    #[must_use]
    pub fn reference(value: Slot) -> Self {
        Self::Reference(Arc::new(Mutex::new(value)))
    }

    /// Create a coarse lock payload for `ConcurrentHashMap`.
    #[must_use]
    pub fn concurrent_map_lock() -> Self {
        Self::ConcurrentMapLock(Arc::new(Mutex::new(())))
    }

    /// Create host-side state for a synthetic `ReentrantLock`.
    #[must_use]
    pub fn reentrant_lock(fair: bool) -> Self {
        Self::ReentrantLock(Arc::new(Mutex::new(ReentrantLockState::new(fair))))
    }

    /// Create host-side state for a synthetic `Condition`.
    #[must_use]
    pub fn condition(lock: Arc<Mutex<ReentrantLockState>>) -> Self {
        Self::Condition(Arc::new(Mutex::new(ConditionState::new(lock))))
    }

    /// Create shared host-side state for a synthetic `ReentrantReadWriteLock`.
    #[must_use]
    pub fn read_write_lock() -> Self {
        Self::ReadWriteLock(Arc::new(Mutex::new(ReadWriteLockState::default())))
    }

    /// Create a read/write view into shared `ReentrantReadWriteLock` state.
    #[must_use]
    pub const fn read_write_lock_view(
        state: Arc<Mutex<ReadWriteLockState>>,
        kind: ReadWriteLockViewKind,
    ) -> Self {
        Self::ReadWriteLockView { state, kind }
    }

    /// Create host-side state for a synthetic `ExecutorService`.
    #[must_use]
    pub fn executor(max_workers: usize) -> Self {
        Self::Executor(Arc::new(ExecutorShared::new(max_workers)))
    }

    /// Create host-side state for a synthetic `CountDownLatch`.
    #[must_use]
    pub fn count_down_latch(count: i32) -> Self {
        Self::CountDownLatch(Arc::new(Mutex::new(CountDownLatchState::new(count))))
    }

    /// Create host-side state for a synthetic `Semaphore`.
    #[must_use]
    pub fn semaphore(permits: i32, fair: bool) -> Self {
        Self::Semaphore(Arc::new(Mutex::new(SemaphoreState::new(permits, fair))))
    }

    /// Create host-side state for a synthetic `CyclicBarrier`.
    #[must_use]
    pub fn cyclic_barrier(parties: i32) -> Self {
        Self::CyclicBarrier(Arc::new(Mutex::new(CyclicBarrierState::new(parties))))
    }

    fn reference_slot(&self) -> Option<Slot> {
        match self {
            Self::Reference(cell) => Some(
                *cell
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            ),
            Self::Int(_)
            | Self::Long(_)
            | Self::Bool(_)
            | Self::ConcurrentMapLock(_)
            | Self::ReentrantLock(_)
            | Self::Condition(_)
            | Self::ReadWriteLock(_)
            | Self::ReadWriteLockView { .. }
            | Self::Executor(_)
            | Self::CountDownLatch(_)
            | Self::Semaphore(_)
            | Self::CyclicBarrier(_) => None,
        }
    }

    pub(crate) fn young_reference_child(&self) -> Option<usize> {
        self.reference_slot()
            .and_then(|slot| slot.as_reference())
            .filter(|r| r & OLD_BIT == 0)
            .and_then(|r| usize::try_from(r).ok())
    }

    pub(crate) fn old_reference_child(&self) -> Option<u64> {
        self.reference_slot()
            .and_then(|slot| slot.as_reference())
            .filter(|r| r & OLD_BIT != 0)
    }

    pub(crate) fn patch_forwarded_reference(&self, forward_map: &HashMap<u64, u64>) -> bool {
        let Self::Reference(cell) = self else {
            return false;
        };
        let mut slot = cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        patch_forwarded_slot(&mut slot, forward_map);
        slot.as_reference().is_some_and(|r| r & OLD_BIT == 0)
    }
}
