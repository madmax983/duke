//! Generational mark-sweep GC for the Duke JVM (Phase 25).
//!
//! **Young generation** — bump-pointer allocation (Eden-style). Minor GC uses
//! a copy-collector: live objects are copied to `to_space`, forwarding pointers
//! patch all live slots, then `to_space` becomes the new young gen.
//!
//! **Old generation** — Phase 24 mark-sweep with free-list reuse.
//!
//! **Reference encoding** — the high bit of every `u64` heap reference indicates
//! generation: `r & OLD_BIT == 0` → young-gen index; `r & OLD_BIT != 0` →
//! old-gen index `(r & !OLD_BIT)`.
//!
//! [`Heap::get`] and [`Heap::get_mut`] are generation-agnostic; callers never
//! need to know which gen an object lives in.
//!
//! ## Examples
//!
//! ```rust
//! use duke_gc::Heap;
//! use duke_runtime::Slot;
//!
//! let mut heap = Heap::new();
//! let string_ref = heap.allocate_string("Duke".to_string());
//! let obj = heap.get(string_ref).unwrap();
//! assert_eq!(obj.string_value.as_deref(), Some("Duke"));
//! ```

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use duke_runtime::{Error, Result, Slot};

pub(crate) mod host;
mod mermaid;
pub use host::{HostFileHandle, HostProcessHandle, SpawnedProcessIds};

/// High bit set ⟹ old-generation reference; clear ⟹ young-generation reference.
pub const OLD_BIT: u64 = 1 << 63;

/// Default number of young-gen slots before a minor GC fires.
const DEFAULT_YOUNG_CAPACITY: usize = 512;

/// Default number of minor-GC survivals before an object is promoted to old gen.
const DEFAULT_PROMOTION_AGE: u8 = 4;

/// Host-side state for synthetic `ReentrantLock` objects.
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

    fn young_reference_child(&self) -> Option<usize> {
        self.reference_slot()
            .and_then(|slot| slot.as_reference())
            .filter(|r| r & OLD_BIT == 0)
            .and_then(|r| usize::try_from(r).ok())
    }

    fn old_reference_child(&self) -> Option<u64> {
        self.reference_slot()
            .and_then(|slot| slot.as_reference())
            .filter(|r| r & OLD_BIT != 0)
    }

    fn patch_forwarded_reference(&self, forward_map: &HashMap<u64, u64>) -> bool {
        let Self::Reference(cell) = self else {
            return false;
        };
        let mut slot = cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        patch_forwarded_slot(&mut slot, forward_map);
        slot.as_reference().is_some_and(|r| r & OLD_BIT == 0)
    }

    /// Rewrite an OLD-gen reference payload through the old-gen compaction map
    /// `old_forward` (old ref → new old ref). Used by [`Heap::compact_old`] so
    /// an `AtomicReference` pointing at a relocated old-gen object stays valid.
    fn patch_old_forwarded_reference(&self, old_forward: &HashMap<u64, u64>) {
        let Self::Reference(cell) = self else {
            return;
        };
        let mut slot = cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        patch_old_forwarded_slot(&mut slot, old_forward);
    }
}

/// A single heap-allocated Java object.
///
/// # Examples
///
/// ```
/// use duke_gc::HeapObject;
/// use duke_runtime::Slot;
///
/// // Usually created via `Heap::allocate`
/// let mut heap = duke_gc::Heap::new();
/// let obj_ref = heap.allocate("java/lang/Object".to_string(), 1);
///
/// let obj = heap.get_mut(obj_ref).unwrap();
/// obj.fields[0] = Slot::Int(42);
/// assert_eq!(obj.class_name, "java/lang/Object");
/// ```
#[derive(Debug, Clone)]
pub struct HeapObject {
    /// The runtime class name of this object (e.g. `"java/lang/String"`).
    pub class_name: String,
    /// Storage for all instance fields of this object.
    pub fields: Vec<Slot>,
    /// String content for `java/lang/String` objects. `None` for non-string objects.
    pub string_value: Option<String>,
    /// Optional host-side atomic backing cell for synthetic atomic objects.
    pub atomic_payload: Option<AtomicPayload>,
    /// Mark bit for old-gen mark-sweep GC. `false` until marked reachable.
    // INVARIANT: Heap::mark_old() traces only `fields` for child references.
    // Any new Vec<Slot> member added to HeapObject MUST also be covered in mark_old().
    pub(crate) marked: bool,
    /// Number of minor GC collections this object has survived.
    #[allow(dead_code)] // used by promote_to_old
    pub(crate) age: u8,
    /// Forwarding pointer installed during the copy phase of minor GC.
    /// `Some(new_ref)` means this object was already copied; `None` means not yet copied.
    #[allow(dead_code)] // used by minor_collect_prepare / apply_forward
    pub(crate) forward: Option<u64>,
    /// Lazily-assigned identity hash (`Object.hashCode` / `System.identityHashCode`).
    /// Assigned from `Heap::next_identity_hash` on first request and carried over
    /// verbatim when the object is copied or promoted, so identity hashes are
    /// STABLE across relocation (unlike the reference index they used to derive from).
    pub(crate) identity_hash: Option<i32>,
}

/// The generational object heap.
///
/// Young-gen refs: `r & OLD_BIT == 0`  → index into `young`
/// Old-gen refs:   `r & OLD_BIT != 0`  → index `(r & !OLD_BIT)` into `old`
///
/// # Examples
///
/// ```
/// use duke_gc::Heap;
/// use duke_runtime::Slot;
///
/// let mut heap = Heap::new();
/// let obj_ref = heap.allocate("MyClass".to_string(), 2);
///
/// let obj = heap.get_mut(obj_ref).unwrap();
/// obj.fields[0] = Slot::Int(42);
///
/// assert_eq!(heap.get(obj_ref).unwrap().fields[0], Slot::Int(42));
/// ```
#[derive(Debug, Default)]
pub struct Heap {
    // ── Young generation ────────────────────────────────────────────────────
    /// Young-gen object store. Index = raw young-gen reference.
    young: Vec<Option<HeapObject>>,
    /// Next free slot in young gen (bump pointer).
    young_top: usize,
    /// Staging area for the copy phase of minor GC; swapped with `young` on finish.
    to_space: Vec<Option<HeapObject>>,

    // ── Old generation ───────────────────────────────────────────────────────
    /// Old-gen object store. Index = `(r & !OLD_BIT)`.
    pub(crate) old: Vec<Option<HeapObject>>,
    /// Free-list of raw old-gen indices (no `OLD_BIT`) for reuse after sweep.
    old_free_list: Vec<u64>,

    // ── GC accounting ────────────────────────────────────────────────────────
    /// Live old-gen objects after the last major GC.
    live_after_last_gc: usize,
    /// Allocations since the last major GC (used for major-GC threshold).
    alloc_since_gc: usize,

    // ── Write barrier ────────────────────────────────────────────────────────
    /// Old-gen raw indices that hold ≥ 1 young-gen reference in their fields.
    /// Populated by `write_field`; cleared by `minor_collect_finish`.
    pub(crate) remembered_set: HashSet<usize>,

    // ── Tuning ───────────────────────────────────────────────────────────────
    /// Minor GC fires when `young_top >= young_capacity`.
    pub young_capacity: usize,
    /// Object is promoted to old gen after surviving this many minor GCs.
    pub(crate) promotion_age: u8,

    // ── Live-count cache ─────────────────────────────────────────────────────
    /// Total live objects across both generations; maintained for O(1) `len()`.
    live_count: usize,

    // ── Collection stats ─────────────────────────────────────────────────────
    /// Young objects dropped (not forwarded) in the most recent minor GC.
    /// Exposed via `free_list_len()` (combined with `old_free_list`) for test
    /// compatibility.
    young_dropped: usize,

    // ── Post-minor-GC forwarding map ─────────────────────────────────────────
    /// Maps old young-gen ref → new ref (young or old-gen with `OLD_BIT`).
    /// Populated during `minor_collect_prepare`, kept alive past
    /// `minor_collect_finish` so callers can patch their own slots after
    /// `collect()` returns via [`Heap::apply_forward`].
    forward_map: HashMap<u64, u64>,

    // ── Identity hashing ─────────────────────────────────────────────────────
    /// Monotonic source for lazily-assigned object identity hashes. Independent
    /// of the reference index, so a hash assigned before relocation survives the
    /// object being copied/promoted. Starts at 1 (0 is reserved so identity
    /// hashes never collide with a "null" sentinel).
    next_identity_hash: i32,

    /// Host OS file handles keyed by small integer ids stored in Java objects.
    pub(crate) host_files: HashMap<i32, host::HostFileHandle>,
    pub(crate) next_host_file_id: i32,
}

impl Heap {
    /// Creates a new heap.
    ///
    /// # Examples
    ///
    /// # Panics
    ///
    /// Panics if `field_count` exceeds 1,048,576 to prevent `OutOfMemory` capacity overflows.
    ///
    /// ```
    /// use duke_gc::Heap;
    /// let heap = Heap::new();
    /// assert!(heap.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            young: Vec::new(),
            young_top: 0,
            to_space: Vec::new(),
            old: Vec::new(),
            old_free_list: Vec::new(),
            live_after_last_gc: 0,
            alloc_since_gc: 0,
            remembered_set: HashSet::new(),
            young_capacity: DEFAULT_YOUNG_CAPACITY,
            promotion_age: DEFAULT_PROMOTION_AGE,
            live_count: 0,
            young_dropped: 0,
            forward_map: HashMap::new(),
            next_identity_hash: 1,
            host_files: HashMap::new(),
            next_host_file_id: 1,
        }
    }

    // ── Allocation ───────────────────────────────────────────────────────────

    const fn make_obj(
        class_name: String,
        fields: Vec<Slot>,
        string_value: Option<String>,
    ) -> HeapObject {
        HeapObject {
            class_name,
            fields,
            string_value,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }
    }

    /// Allocate a new object in the young generation. Returns a young-gen reference.
    ///
    /// # Panics
    ///
    /// Panics if `field_count` exceeds 1,048,576 to prevent `OutOfMemory` capacity overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// let r = heap.allocate("java/lang/Object".to_string(), 0);
    /// assert_eq!(heap.get(r).unwrap().class_name, "java/lang/Object");
    /// ```
    pub fn allocate(&mut self, class_name: String, field_count: usize) -> u64 {
        // 👺 HAVOC: Stop OOM attacks!
        // Prevent unbounded allocation which triggers stdlib capacity overflow and crashes the process.
        // We explicitly assert limit. A "graceful" panic provides a backtrace and prevents
        // uncontrolled heap exhaustions inside the VM execution layer.
        assert!(
            field_count <= 1024 * 1024,
            "Allocation exceeded maximum allowed field count"
        );
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64; // OLD_BIT == 0 → young ref
        let obj = Self::make_obj(class_name, vec![Slot::Int(0); field_count], None);
        self.young.push(Some(obj));
        self.young_top += 1;
        idx
    }

    /// Allocate a new String object in the young generation. Returns a young-gen reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// let r = heap.allocate_string("Hello".to_string());
    /// assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("Hello"));
    /// ```
    pub fn allocate_string(&mut self, value: String) -> u64 {
        self.alloc_since_gc += 1;
        self.live_count += 1;
        let idx = self.young_top as u64;
        // Real java/lang/String layout: slot0 value:[B, slot1 coder:B,
        // slot2 hash:I, slot3 hashIsZero:Z. slot0 is filled by
        // `set_string_layout`; slots 1-3 start at 0. `string_value` remains the
        // authoritative side-channel that the ~285 existing readers use.
        let fields = vec![
            Slot::Reference(None),
            Slot::Int(0),
            Slot::Int(0),
            Slot::Int(0),
        ];
        // Move `value` into the object as its authoritative `string_value`
        // side-channel; the real-layout slots are then populated by re-reading
        // that stored payload, single-sourcing the encoding through
        // `set_string_layout` rather than cloning `value` here.
        let obj = Self::make_obj("java/lang/String".to_string(), fields, Some(value));
        self.young.push(Some(obj));
        self.young_top += 1;
        let contents = self
            .get(idx)
            .ok()
            .and_then(|obj| obj.string_value.clone())
            .unwrap_or_default();
        self.set_string_layout(idx, &contents);
        idx
    }

    /// Populates the real `java/lang/String` layout slots for the String object
    /// at `string_ref`, allocating a backing `[B` byte array for slot 0 (`value`)
    /// and writing the coder into slot 1 (`coder`).
    ///
    /// The byte array uses Latin-1 encoding when every char is `<= 0xFF`
    /// (`coder = 0`), otherwise little-endian UTF-16 (`coder = 1`). Slots 2
    /// (`hash`) and 3 (`hashIsZero`) are left untouched at 0. The authoritative
    /// `string_value` side-channel is not modified here.
    ///
    /// The slot-0 reference store goes through [`Heap::write_field`] so the
    /// generational write barrier fires if `string_ref` has already been
    /// promoted to old gen and the freshly allocated byte array is young.
    ///
    /// # Panics
    ///
    /// Panics if `string_ref` does not refer to a live object whose slots 0 and
    /// 1 are writable, or if the freshly allocated backing byte array cannot be
    /// found immediately after allocation (both indicate heap corruption).
    pub fn set_string_layout(&mut self, string_ref: u64, value: &str) {
        let latin1 = value.chars().all(|c| c as u32 <= 0xFF);
        let (coder, bytes): (i32, Vec<u8>) = if latin1 {
            (0, value.chars().map(|c| c as u8).collect())
        } else {
            (1, value.encode_utf16().flat_map(u16::to_le_bytes).collect())
        };

        let bytes_ref = self.allocate("[B".to_string(), bytes.len());
        {
            let arr = self
                .get_mut(bytes_ref)
                .expect("freshly allocated byte array must exist");
            for (i, &b) in bytes.iter().enumerate() {
                // Java `byte` is signed: reinterpret the raw octet as i8.
                arr.fields[i] = Slot::Int(i32::from(i8::from_ne_bytes([b])));
            }
        }

        // slot0 value:[B — via write_field so the old→young barrier fires.
        self.write_field(string_ref, 0, Slot::Reference(Some(bytes_ref)))
            .expect("String slot 0 (value) must be writable");
        // slot1 coder:B
        self.write_field(string_ref, 1, Slot::Int(coder))
            .expect("String slot 1 (coder) must be writable");
    }

    /// Finds a live object by runtime class and string payload.
    ///
    /// This is intentionally narrow: the interpreter uses it to canonicalize
    /// VM metadata objects such as `java/lang/Class`, which are represented as
    /// string-backed heap objects.
    #[must_use]
    pub fn find_string_backed_object(&self, class_name: &str, value: &str) -> Option<u64> {
        for (idx, slot) in self.young.iter().enumerate() {
            let Some(obj) = slot else {
                continue;
            };
            if obj.class_name == class_name && obj.string_value.as_deref() == Some(value) {
                return Some(idx as u64);
            }
        }
        for (idx, slot) in self.old.iter().enumerate() {
            let Some(obj) = slot else {
                continue;
            };
            if obj.class_name == class_name && obj.string_value.as_deref() == Some(value) {
                return Some(idx as u64 | OLD_BIT);
            }
        }
        None
    }

    /// Clones an existing object in the heap. Returns the reference of the new object.
    ///
    /// # Errors
    /// Returns `Error::NullPointerException` if the source reference is invalid.
    pub fn clone_object(&mut self, src_ref: u64) -> Result<u64> {
        let src = self.get(src_ref)?;
        let class_name = src.class_name.clone();
        let fields = src.fields.clone();
        let string_value = src.string_value.clone();
        let atomic_payload = src.atomic_payload.clone();

        let new_ref = self.allocate(class_name, 0);
        let dest = self.get_mut(new_ref)?;
        dest.fields = fields;
        dest.string_value = string_value;
        dest.atomic_payload = atomic_payload;
        Ok(new_ref)
    }

    /// Returns the stable identity hash for the object at `r`, assigning one on
    /// first request from a monotonic counter.
    ///
    /// The hash is stored on the object and carried over verbatim when the object
    /// is copied to `to_space` or promoted to old gen, so it is **stable across
    /// relocation** — unlike a hash derived from the (mutable) reference index.
    /// This is the accessor `Object.hashCode` / `System.identityHashCode` /
    /// `Objects.hashCode` must use once the collector can move objects.
    ///
    /// # Errors
    /// Returns [`Error::InvalidRef`] if `r` does not name a live object.
    pub fn identity_hash(&mut self, r: u64) -> Result<i32> {
        if let Some(h) = self.get(r)?.identity_hash {
            return Ok(h);
        }
        let h = self.next_identity_hash;
        // Advance, skipping 0 on wrap so the counter never yields the sentinel.
        self.next_identity_hash = match self.next_identity_hash.wrapping_add(1) {
            0 => 1,
            n => n,
        };
        self.get_mut(r)?.identity_hash = Some(h);
        Ok(h)
    }

    /// Promote a young-gen object to old gen. Returns `raw_old_idx | OLD_BIT`.
    fn promote_to_old(&mut self, mut obj: HeapObject) -> u64 {
        obj.age = 0; // reset age in old gen (not used there)
        obj.forward = None;
        if let Some(raw_idx) = self.old_free_list.pop() {
            self.old[usize::try_from(raw_idx).unwrap()] = Some(obj);
            raw_idx | OLD_BIT
        } else {
            let raw_idx = self.old.len() as u64;
            self.old.push(Some(obj));
            raw_idx | OLD_BIT
        }
    }

    // ── Object access ────────────────────────────────────────────────────────

    fn young_index_from_ref(r: u64) -> Result<usize> {
        usize::try_from(r).map_err(|_| Error::InvalidRef { address: r })
    }

    fn old_index_from_ref(r: u64) -> Result<usize> {
        usize::try_from(r & !OLD_BIT).map_err(|_| Error::InvalidRef { address: r })
    }

    /// Returns a reference to the object at `r`, dispatching on `OLD_BIT`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidRef`] if `r` is out of bounds or the slot is `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// let obj_ref = heap.allocate("MyClass".to_string(), 0);
    /// assert_eq!(heap.get(obj_ref).unwrap().class_name, "MyClass");
    /// ```
    pub fn get(&self, r: u64) -> Result<&HeapObject> {
        if r & OLD_BIT != 0 {
            let idx = Self::old_index_from_ref(r)?;
            if idx >= self.old.len() {
                return Err(Error::InvalidRef { address: r });
            }
            self.old
                .get(idx)
                .and_then(|s| s.as_ref())
                .ok_or(Error::InvalidRef { address: r })
        } else {
            let idx = Self::young_index_from_ref(r)?;
            if idx >= self.young.len() {
                return Err(Error::InvalidRef { address: r });
            }
            self.young
                .get(idx)
                .and_then(|s| s.as_ref())
                .ok_or(Error::InvalidRef { address: r })
        }
    }

    /// Returns a mutable reference to the object at `r`, dispatching on `OLD_BIT`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidRef`] if `r` is out of bounds or the slot is `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    /// use duke_runtime::Slot;
    ///
    /// let mut heap = Heap::new();
    /// let obj_ref = heap.allocate("MyClass".to_string(), 1);
    /// heap.get_mut(obj_ref).unwrap().fields[0] = Slot::Int(123);
    /// ```
    pub fn get_mut(&mut self, r: u64) -> Result<&mut HeapObject> {
        if r & OLD_BIT != 0 {
            let idx = Self::old_index_from_ref(r)?;
            if idx >= self.old.len() {
                return Err(Error::InvalidRef { address: r });
            }
            self.old
                .get_mut(idx)
                .and_then(|s| s.as_mut())
                .ok_or(Error::InvalidRef { address: r })
        } else {
            let idx = Self::young_index_from_ref(r)?;
            if idx >= self.young.len() {
                return Err(Error::InvalidRef { address: r });
            }
            self.young
                .get_mut(idx)
                .and_then(|s| s.as_mut())
                .ok_or(Error::InvalidRef { address: r })
        }
    }

    // ── Heap stats ───────────────────────────────────────────────────────────

    /// Returns the total number of live objects across both generations.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let mut heap = Heap::new();
    /// assert_eq!(heap.len(), 0);
    /// heap.allocate("MyClass".to_string(), 0);
    /// assert_eq!(heap.len(), 1);
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        self.live_count
    }

    /// Returns whether the heap is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    ///
    /// let heap = Heap::new();
    /// assert!(heap.is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Returns the number of live objects in the old generation.
    #[must_use]
    pub fn old_live_count(&self) -> usize {
        self.old.iter().filter(|s| s.is_some()).count()
    }

    // ── GC triggers ──────────────────────────────────────────────────────────

    /// Returns `true` when the young gen is full (minor GC should fire).
    #[must_use]
    pub const fn should_minor_gc(&self) -> bool {
        self.young_top >= self.young_capacity
    }

    /// Returns `true` when the old gen has grown to 2× its post-GC size.
    /// The minimum threshold is 256 allocations (prevents thrashing on tiny heaps).
    #[must_use]
    pub fn should_major_gc(&self) -> bool {
        let threshold = (self.live_after_last_gc * 2).max(256);
        self.alloc_since_gc >= threshold
    }

    /// Returns `true` if any young-gen objects have been forwarded by an
    /// in-progress minor GC.  Used by callers to gate the `apply_forward`
    /// sweep: when `false`, the forward map is empty and every `apply_forward`
    /// call is a no-op, so the sweep can be skipped entirely.
    #[must_use]
    pub fn has_pending_forwards(&self) -> bool {
        !self.forward_map.is_empty()
    }

    /// Compatibility shim — use `should_minor_gc` / `should_major_gc` instead.
    #[must_use]
    pub fn should_gc(&self) -> bool {
        self.should_major_gc()
    }

    // ── Write barrier ────────────────────────────────────────────────────────

    /// Records a reference write into non-field object storage, such as an atomic reference payload.
    pub fn remember_reference_write(&mut self, obj_ref: u64, value: Slot) {
        if obj_ref & OLD_BIT != 0
            && value.as_reference().is_some_and(|r| r & OLD_BIT == 0)
            && let Ok(idx) = Self::old_index_from_ref(obj_ref)
        {
            self.remembered_set.insert(idx);
        }
    }

    /// Store `value` into field `field_idx` of the object at `obj_ref`.
    ///
    /// If the target object is in old gen and `value` is a young-gen reference,
    /// the old-gen object's raw index is added to the remembered set so the
    /// minor GC will scan it for cross-generational pointers.
    ///
    /// # Errors
    /// Returns [`Error::InvalidRef`] if `obj_ref` is invalid.
    /// Returns [`Error::FieldOutOfBounds`] if `field_idx` is not a valid field.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_gc::Heap;
    /// use duke_runtime::Slot;
    ///
    /// let mut heap = Heap::new();
    /// let obj_ref = heap.allocate("Box".to_string(), 1);
    /// heap.write_field(obj_ref, 0, Slot::Int(42)).unwrap();
    /// assert_eq!(heap.get(obj_ref).unwrap().fields[0], Slot::Int(42));
    /// ```
    pub fn write_field(&mut self, obj_ref: u64, field_idx: usize, value: Slot) -> Result<()> {
        let remember_old_idx =
            if obj_ref & OLD_BIT != 0 && value.as_reference().is_some_and(|r| r & OLD_BIT == 0) {
                Some(Self::old_index_from_ref(obj_ref)?)
            } else {
                None
            };

        {
            let obj = self.get_mut(obj_ref)?;
            let length = obj.fields.len();
            let field = obj
                .fields
                .get_mut(field_idx)
                .ok_or(Error::FieldOutOfBounds {
                    index: field_idx,
                    length,
                })?;
            *field = value;
        }

        if let Some(idx) = remember_old_idx {
            self.remembered_set.insert(idx);
        }
        Ok(())
    }

    // ── Minor GC (copy collector) ────────────────────────────────────────────

    /// Clone an `Arc` to every synthetic executor's shared state currently
    /// reachable through a heap object's atomic payload.
    ///
    /// The clones are held for the duration of a minor GC so the executor task
    /// queues can be treated as an extra root set (their `future_ref`/`task_ref`
    /// entries are bare `u64`s the mutator never sees) and patched after
    /// forwarding — even if the owning executor object is itself collected this
    /// cycle (worker threads keep the `Arc` alive regardless).
    fn collect_executor_shared(&self) -> Vec<Arc<ExecutorShared>> {
        self.young
            .iter()
            .chain(self.old.iter())
            .flatten()
            .filter_map(|obj| match &obj.atomic_payload {
                Some(AtomicPayload::Executor(shared)) => Some(Arc::clone(shared)),
                _ => None,
            })
            .collect()
    }

    /// Seed `worklist` with the young-gen refs held in each executor's task queue.
    fn seed_executor_queue_roots(executors: &[Arc<ExecutorShared>], worklist: &mut Vec<usize>) {
        for shared in executors {
            let guard = shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for task in &guard.queue {
                for r in [task.future_ref, task.task_ref] {
                    if r & OLD_BIT == 0 {
                        worklist.push(usize::try_from(r).unwrap());
                    }
                }
            }
        }
    }

    /// Rewrite executor task-queue refs through the forwarding map so queued
    /// tasks whose objects were copied or promoted this cycle stay valid.
    /// Idempotent: already-forwarded refs are absent from `forward_map`.
    fn patch_executor_queue_refs(&self, executors: &[Arc<ExecutorShared>]) {
        if self.forward_map.is_empty() {
            return;
        }
        for shared in executors {
            let mut guard = shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for task in &mut guard.queue {
                if let Some(&nr) = self.forward_map.get(&task.future_ref) {
                    task.future_ref = nr;
                }
                if let Some(&nr) = self.forward_map.get(&task.task_ref) {
                    task.task_ref = nr;
                }
            }
        }
    }

    /// **Phase 1 of minor GC**: trace live young objects from `roots` and the
    /// remembered set, copy them to `to_space`, and install forwarding pointers.
    ///
    /// Call [`Heap::apply_forward`] on every live interpreter slot after this,
    /// then call [`Heap::minor_collect_finish`] to complete the collection.
    ///
    /// # Panics
    /// Panics if a young-gen reference in `roots` cannot be converted to `usize`,
    /// which cannot happen on 64-bit targets since heap indices are always small.
    pub fn minor_collect_prepare(&mut self, roots: &[Slot]) {
        // ⚡ Bolt: Pre-allocate `to_space` based on maximum possible survivors to eliminate dynamic resizing overhead.
        self.to_space = Vec::with_capacity(self.young.len());
        self.forward_map.clear();

        // Host-side executor task queues hold bare `u64` refs that are not part
        // of the mutator root set. Snapshot the shared states up-front so we can
        // both seed them as roots below and patch them after forwarding.
        let executors = self.collect_executor_shared();

        // Seed worklist with young refs from roots and remembered-set fields.
        // ⚡ Bolt: Pre-allocate `worklist` based on root set size to eliminate initial dynamic resizing overhead.
        let mut worklist: Vec<usize> = Vec::with_capacity(roots.len());

        for slot in roots {
            if let Some(r) = slot.as_reference()
                && r & OLD_BIT == 0
            {
                worklist.push(usize::try_from(r).unwrap());
            }
        }

        // Scan remembered-set old-gen objects for young refs.
        // ⚡ Bolt: Avoid intermediate vector allocation by iterating directly
        for &old_idx in &self.remembered_set {
            if let Some(Some(obj)) = self.old.get(old_idx) {
                worklist.extend(
                    obj.fields
                        .iter()
                        .filter_map(Slot::as_reference)
                        .filter(|r| r & OLD_BIT == 0)
                        .map(|r| usize::try_from(r).unwrap()),
                );
                if let Some(child) = obj
                    .atomic_payload
                    .as_ref()
                    .and_then(AtomicPayload::young_reference_child)
                {
                    worklist.push(child);
                }
            }
        }

        // Seed young refs held in executor task queues so queued-but-unrun tasks
        // (and the futures they report into) survive this collection.
        Self::seed_executor_queue_roots(&executors, &mut worklist);

        // Copy phase — BFS worklist.
        while let Some(y_idx) = worklist.pop() {
            let is_forwarded = self
                .young
                .get(y_idx)
                .and_then(|o| o.as_ref())
                .is_some_and(|o| o.forward.is_some());
            if is_forwarded {
                continue;
            }

            let Some(Some(mut copy)) = self.young.get_mut(y_idx).map(std::option::Option::take)
            else {
                continue;
            };

            // Push young children onto worklist *before* moving copy
            worklist.extend(
                copy.fields
                    .iter()
                    .filter_map(Slot::as_reference)
                    .filter(|r| r & OLD_BIT == 0)
                    .map(|r| usize::try_from(r).unwrap()),
            );
            if let Some(child) = copy
                .atomic_payload
                .as_ref()
                .and_then(AtomicPayload::young_reference_child)
            {
                worklist.push(child);
            }

            let new_ref = if copy.age >= self.promotion_age {
                self.promote_to_old(copy)
            } else {
                copy.age += 1;
                let new_idx = self.to_space.len() as u64;
                self.to_space.push(Some(copy));
                new_idx
            };

            // Install dummy object with forwarding pointer
            self.young[y_idx] = Some(HeapObject {
                class_name: String::new(),
                fields: vec![],
                string_value: None,
                atomic_payload: None,
                marked: false,
                age: 0,
                forward: Some(new_ref),
                identity_hash: None,
            });
            self.forward_map.insert(y_idx as u64, new_ref);
        }

        // Patch survivors and old-gen objects, and rebuild the remembered set.
        self.patch_survivors_and_old_gen();

        // Rewrite executor task-queue refs to their forwarded locations.
        self.patch_executor_queue_refs(&executors);
    }

    /// Post-copy patch pass: rewrite intra-young references in the copied
    /// survivors, then patch every old-gen object and rebuild the remembered set
    /// so old→young edges (including those from newly promoted objects) survive
    /// the next minor GC.
    fn patch_survivors_and_old_gen(&mut self) {
        let forward_map = &self.forward_map;

        // Patch copied young survivors so their intra-young references point at
        // the forwarded children instead of stale from-space indices.
        for obj in self.to_space.iter_mut().flatten() {
            patch_forwarded_fields(&mut obj.fields, forward_map);
            if let Some(payload) = &obj.atomic_payload {
                payload.patch_forwarded_reference(forward_map);
            }
        }

        // Patch all old-gen objects and rebuild the remembered set so existing
        // old->young edges, including newly promoted objects, survive the next minor GC.
        let mut rebuilt_remembered_set = HashSet::new();
        for (old_idx, obj) in self.old.iter_mut().enumerate() {
            let Some(obj) = obj.as_mut() else {
                continue;
            };
            let fields_have_young = patch_forwarded_fields(&mut obj.fields, forward_map);
            let atomic_has_young = obj
                .atomic_payload
                .as_ref()
                .is_some_and(|payload| payload.patch_forwarded_reference(forward_map));
            if fields_have_young || atomic_has_young {
                rebuilt_remembered_set.insert(old_idx);
            }
        }
        self.remembered_set = rebuilt_remembered_set;
    }

    /// Patch a single slot to point to the forwarded address, if applicable.
    ///
    /// Works both during `minor_collect_prepare` (reads from `young[].forward`)
    /// and after `minor_collect_finish` (reads from `forward_map`). No-op if
    /// the slot is not a young-gen reference or has no forwarding pointer.
    ///
    /// # Panics
    /// Panics if the young-gen reference value cannot be converted to `usize`,
    /// which cannot happen on 64-bit targets since heap indices are always small.
    pub fn apply_forward(&self, slot: &mut Slot) {
        let Some(r) = slot.as_reference() else {
            return;
        };
        if r & OLD_BIT == 0 {
            // Young ref. Try forward_map first (valid at any phase); fall back to
            // young[].forward if forward_map hasn't been populated yet for this ref.
            let new_r = self.forward_map.get(&r).copied().or_else(|| {
                self.young
                    .get(usize::try_from(r).unwrap())
                    .and_then(|s| s.as_ref())
                    .and_then(|o| o.forward)
            });
            if let Some(nr) = new_r {
                *slot = Slot::Reference(Some(nr));
            }
        } else if let Some(&nr) = self.forward_map.get(&r) {
            // Old ref. `forward_map` only holds OLD_BIT-tagged keys after an
            // old-gen compaction (`compact_old`); without compaction this branch
            // never fires, preserving the minor-only fast path exactly.
            *slot = Slot::Reference(Some(nr));
        }
    }

    /// **Phase 3 of minor GC**: swap `to_space` into `young` and reset `young_top`.
    pub fn minor_collect_finish(&mut self) {
        // Recalculate live_count: survivors in to_space + live old-gen objects.
        let young_live = self.to_space.iter().filter(|s| s.is_some()).count();
        let old_live = self.old.iter().filter(|s| s.is_some()).count();

        // Count young objects that had no forwarding pointer (dropped).
        let young_alive_before = self.young.iter().filter(|s| s.is_some()).count();
        // Forwarded = promoted_to_old + copied_to_to_space; dropped = was live but not forwarded.
        // We compute it as: (objects that were Some in young) - (those that got a forward pointer).
        let forwarded = self
            .young
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|o| o.forward.is_some())
            .count();
        self.young_dropped = young_alive_before.saturating_sub(forwarded);

        self.young = std::mem::take(&mut self.to_space);
        self.young_top = self.young.len();
        self.live_count = young_live + old_live;
    }

    // ── Major GC (old-gen mark-sweep) ────────────────────────────────────────

    /// Mark-sweep the old generation. Only old-gen roots (`OLD_BIT` set) are
    /// traced. Young-gen survivors must be promoted before calling this.
    pub fn major_collect(&mut self, roots: &[Slot]) {
        self.mark_old(roots);
        self.sweep_old();
    }

    fn mark_old(&mut self, roots: &[Slot]) {
        let mut worklist: Vec<u64> = roots
            .iter()
            .filter_map(Slot::as_reference)
            .filter(|r| r & OLD_BIT != 0)
            .collect();

        while let Some(r) = worklist.pop() {
            let Ok(idx) = Self::old_index_from_ref(r) else {
                continue;
            };
            let Some(Some(obj)) = self.old.get_mut(idx) else {
                continue;
            };
            if obj.marked {
                continue;
            }
            obj.marked = true;
            let children: Vec<u64> = obj
                .fields
                .iter()
                .filter_map(Slot::as_reference)
                .filter(|c| c & OLD_BIT != 0)
                .collect();
            worklist.extend(children);
            if let Some(child) = obj
                .atomic_payload
                .as_ref()
                .and_then(AtomicPayload::old_reference_child)
            {
                worklist.push(child);
            }
        }
    }

    fn sweep_old(&mut self) {
        let mut live = 0usize;
        for (idx, slot) in self.old.iter_mut().enumerate() {
            match slot {
                Some(obj) if obj.marked => {
                    obj.marked = false;
                    live += 1;
                }
                Some(_) => {
                    *slot = None;
                    self.old_free_list.push(idx as u64);
                }
                None => {}
            }
        }
        self.live_after_last_gc = live;
        self.alloc_since_gc = 0;
        let young_live = self.young.iter().filter(|s| s.is_some()).count();
        self.live_count = young_live + live;
    }

    /// Full collection: run a minor GC first (promotes all young survivors to old),
    /// then run old-gen mark-sweep.
    ///
    /// **Caller must apply forwarding pointers to all live interpreter slots
    /// between `minor_collect_prepare` and `minor_collect_finish` before calling
    /// this.** The `collect` shim handles this internally for tests/compat callers
    /// by building a patched-roots vec.
    pub fn collect(&mut self, roots: &[Slot]) {
        // Step 1: promote all young survivors to old gen.
        self.minor_collect_prepare(roots);

        // Build patched roots for the major GC by applying forwarding pointers.
        let mut patched: Vec<Slot> = roots.to_vec();
        for slot in &mut patched {
            self.apply_forward(slot);
        }

        self.minor_collect_finish();

        // Step 2: mark-sweep old gen (cheap, non-moving).
        self.major_collect(&patched);

        // Step 3: if the old-gen free list has fragmented past the threshold,
        // slide-compact it. Compaction seeds `forward_map` with OLD_BIT-tagged
        // old→old forwards (composed with any young→old forwards from step 1),
        // so the caller's existing `apply_forward` sweep patches every root that
        // points into the old gen — no separate old-gen root walk is needed.
        if self.should_compact_old() {
            self.compact_old(&patched);
        }
    }

    // ── Major GC (old-gen mark-compact / Lisp-2 slide) ───────────────────────

    /// Fraction of the old-gen backing store that is currently dead (free-list
    /// holes). Ranges `0.0..=1.0`.
    ///
    /// **Metric:** `old_free_slots / old_total_slots`. Every `None` hole in the
    /// `old` Vec corresponds to exactly one `old_free_list` entry (holes are
    /// produced by `sweep_old` and consumed by `promote_to_old`), so the free
    /// list length is the dead-slot count. This slot-granular ratio is the
    /// natural fragmentation signal for Duke's per-object free list: a high
    /// value means live objects are sparsely scattered among reusable holes,
    /// which is exactly what mark-compact defeats. Returns `0.0` for an empty
    /// old gen.
    #[must_use]
    pub fn fragmentation_ratio(&self) -> f64 {
        let total = self.old.len();
        if total == 0 {
            return 0.0;
        }
        // total is a live-object count bounded well under 2^52, so the casts are
        // exact on all supported targets.
        #[allow(clippy::cast_precision_loss)]
        let ratio = self.old_free_list.len() as f64 / total as f64;
        ratio
    }

    /// Returns `true` when the old gen is fragmented enough to justify a
    /// (relatively expensive) compacting collection: at least
    /// [`Self::OLD_COMPACT_MIN_SLOTS`] slots and a
    /// [`fragmentation_ratio`](Self::fragmentation_ratio) at or above
    /// [`Self::OLD_COMPACT_FRAGMENTATION_THRESHOLD`].
    #[must_use]
    pub fn should_compact_old(&self) -> bool {
        self.old.len() >= Self::OLD_COMPACT_MIN_SLOTS
            && self.fragmentation_ratio() >= Self::OLD_COMPACT_FRAGMENTATION_THRESHOLD
    }

    /// Fragmentation ratio (dead old slots / total old slots) at or above which
    /// a major collection upgrades from cheap mark-sweep to sliding compaction.
    /// Tuned so compaction only fires once roughly half the old gen is holes.
    pub const OLD_COMPACT_FRAGMENTATION_THRESHOLD: f64 = 0.5;

    /// Minimum old-gen slot count before compaction is considered, so tiny heaps
    /// never pay the compaction walk over a handful of objects.
    pub const OLD_COMPACT_MIN_SLOTS: usize = 64;

    /// Run a major collection that always finishes with a sliding compaction of
    /// the old gen, regardless of the fragmentation threshold. Mirrors
    /// [`collect`](Self::collect) (minor promote → mark-sweep → compact) and is
    /// primarily a deterministic entry point for tests and observability.
    ///
    /// Like `collect`, the caller must apply forwarding to its own roots after
    /// this returns (via [`apply_forward`](Self::apply_forward)); the `Slot`
    /// roots passed here are only used internally.
    pub fn major_collect_compacting(&mut self, roots: &[Slot]) {
        self.minor_collect_prepare(roots);
        let mut patched: Vec<Slot> = roots.to_vec();
        for slot in &mut patched {
            self.apply_forward(slot);
        }
        self.minor_collect_finish();
        self.compact_old(&patched);
    }

    /// Lisp-2 sliding mark-compact of the old generation.
    ///
    /// 1. **Mark** live old objects reachable from `roots` (reuses `` `mark_old` ``).
    /// 2. **Forward** — assign each live object a new, densely-packed old index
    ///    in ascending (stable, sliding) order; record old→new in an
    ///    `old_forward` map (only for objects that actually move).
    /// 3. **Update pointers** everywhere an OLD-gen ref can hide: every live old
    ///    object's fields + atomic payload, every young object's fields + atomic
    ///    payload (young→old edges), the remembered set (index remap), and the
    ///    executor task-queue snapshot. The map is also folded into
    ///    `forward_map` — existing young→old values are re-pointed and direct
    ///    old→old entries added — so the interpreter's post-collection
    ///    `apply_forward` sweep rewrites external roots with no extra machinery.
    /// 4. **Move** survivors into their compacted slots, truncate `old` to the
    ///    live count, and clear the free list (fragmentation → 0).
    ///
    /// `identity_hash` and `atomic_payload` ride along with each moved object,
    /// preserving the [`HeapObject`] invariant across relocation.
    pub fn compact_old(&mut self, roots: &[Slot]) {
        // Snapshot executor shared state up front: their task queues hold bare
        // OLD refs the mutator never sees, so we both (a) treat them as extra
        // roots below — otherwise a queued-but-unrun task's target could be
        // dropped and its forwarded queue ref would dangle — and (b) rewrite
        // them after forwarding.
        let executors = self.collect_executor_shared();

        // ── 1. Mark live old objects (roots + executor-queue OLD refs). ──────
        let mut mark_roots: Vec<Slot> = roots.to_vec();
        for shared in &executors {
            let guard = shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for task in &guard.queue {
                for r in [task.future_ref, task.task_ref] {
                    if r & OLD_BIT != 0 {
                        mark_roots.push(Slot::Reference(Some(r)));
                    }
                }
            }
        }
        self.mark_old(&mark_roots);

        // ── 2. Assign new compacted indices in stable ascending order. ───────
        // new_index[old_idx] = Some(new_idx) for live objects, None otherwise.
        let mut new_index: Vec<Option<usize>> = vec![None; self.old.len()];
        let mut next: usize = 0;
        for (idx, slot) in self.old.iter().enumerate() {
            if slot.as_ref().is_some_and(|obj| obj.marked) {
                new_index[idx] = Some(next);
                next += 1;
            }
        }

        // old_forward: OLD_BIT-tagged old ref → new old ref, only where moved.
        let mut old_forward: HashMap<u64, u64> = HashMap::new();
        for (idx, entry) in new_index.iter().enumerate() {
            if let Some(new_idx) = *entry {
                let old_ref = idx as u64 | OLD_BIT;
                let new_ref = new_idx as u64 | OLD_BIT;
                if old_ref != new_ref {
                    old_forward.insert(old_ref, new_ref);
                }
            }
        }

        // ── 3. Update pointers (in place; ref values are position-independent).
        // (a) live old objects' fields + atomic payload.
        for obj in self.old.iter_mut().flatten() {
            if !obj.marked {
                continue;
            }
            patch_old_forwarded_fields(&mut obj.fields, &old_forward);
            if let Some(payload) = &obj.atomic_payload {
                payload.patch_old_forwarded_reference(&old_forward);
            }
        }
        // (b) young objects' fields + atomic payload (young→old edges).
        for obj in self.young.iter_mut().flatten() {
            patch_old_forwarded_fields(&mut obj.fields, &old_forward);
            if let Some(payload) = &obj.atomic_payload {
                payload.patch_old_forwarded_reference(&old_forward);
            }
        }
        // (c) remembered set — remap surviving old indices to their new slots.
        self.remembered_set = self
            .remembered_set
            .iter()
            .filter_map(|&old_idx| new_index.get(old_idx).copied().flatten())
            .collect();
        // (d) executor task-queue snapshot (bare u64 refs the mutator never sees).
        for shared in &executors {
            let mut guard = shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for task in &mut guard.queue {
                if let Some(&nr) = old_forward.get(&task.future_ref) {
                    task.future_ref = nr;
                }
                if let Some(&nr) = old_forward.get(&task.task_ref) {
                    task.task_ref = nr;
                }
            }
        }
        // (e) fold into forward_map so the interpreter's apply_forward patches
        // external roots. First re-point existing (young→old) values that moved,
        // then add the direct old→old forwards. Old keys never collide with the
        // young keys already present.
        for value in self.forward_map.values_mut() {
            if let Some(&nv) = old_forward.get(value) {
                *value = nv;
            }
        }
        for (old_ref, new_ref) in &old_forward {
            self.forward_map.insert(*old_ref, *new_ref);
        }

        // ── 4. Move survivors into a dense store and rebuild bookkeeping. ─────
        let mut compacted: Vec<Option<HeapObject>> = Vec::with_capacity(next);
        compacted.resize_with(next, || None);
        for (idx, slot) in self.old.iter_mut().enumerate() {
            let Some(new_idx) = new_index[idx] else {
                continue; // dead (unmarked) or empty hole — dropped.
            };
            if let Some(mut obj) = slot.take() {
                obj.marked = false; // clear mark; live outside a GC is unmarked.
                compacted[new_idx] = Some(obj);
            }
        }
        self.old = compacted;
        // Compaction eliminates fragmentation: the store is now a dense prefix,
        // so the free list is empty and future promotions bump the tail.
        self.old_free_list.clear();

        // Refresh live accounting (young survivors + compacted old live).
        let young_live = self.young.iter().filter(|s| s.is_some()).count();
        self.live_after_last_gc = next;
        self.live_count = young_live + next;
    }

    // ── Test helpers ─────────────────────────────────────────────────────────

    /// Number of collected slots: old-gen free list + young objects dropped in
    /// the most recent minor GC. This combined count is used by tests that
    /// verify reclamation without caring which generation the objects lived in.
    #[cfg(test)]
    #[must_use]
    pub const fn free_list_len(&self) -> usize {
        self.old_free_list.len() + self.young_dropped
    }

    /// Total number of old-gen backing slots (live objects + free-list holes).
    /// Exposed for observability: compaction shrinks this to the live count.
    #[must_use]
    pub const fn old_slot_count(&self) -> usize {
        self.old.len()
    }

    /// Generates a Mermaid JS graph of the heap.
    #[must_use]
    pub fn dump_mermaid(&self) -> String {
        mermaid::dump_mermaid(self)
    }
}

fn patch_forwarded_slot(slot: &mut Slot, forward_map: &HashMap<u64, u64>) {
    if let Some(r) = slot.as_reference()
        && r & OLD_BIT == 0
        && let Some(new_r) = forward_map.get(&r).copied()
    {
        *slot = Slot::Reference(Some(new_r));
    }
}

/// Rewrite a single OLD-gen reference slot through an old-gen compaction map
/// (`old_forward`: OLD_BIT-tagged old ref → new old ref). No-op for young refs,
/// null, non-refs, or old refs that did not move.
fn patch_old_forwarded_slot(slot: &mut Slot, old_forward: &HashMap<u64, u64>) {
    if let Some(r) = slot.as_reference()
        && r & OLD_BIT != 0
        && let Some(new_r) = old_forward.get(&r).copied()
    {
        *slot = Slot::Reference(Some(new_r));
    }
}

/// Rewrite every OLD-gen reference in `fields` through the compaction map.
fn patch_old_forwarded_fields(fields: &mut [Slot], old_forward: &HashMap<u64, u64>) {
    for slot in fields {
        patch_old_forwarded_slot(slot, old_forward);
    }
}

fn patch_forwarded_fields(fields: &mut [Slot], forward_map: &HashMap<u64, u64>) -> bool {
    let mut contains_young_ref = false;
    for slot in fields {
        patch_forwarded_slot(slot, forward_map);
        if slot.as_reference().is_some_and(|r| r & OLD_BIT == 0) {
            contains_young_ref = true;
        }
    }
    contains_young_ref
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_return_error_when_reading_closed_or_invalid_file() {
        let mut gc = Heap::new();
        let err = gc.read_host_file_byte(999).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn should_return_error_when_writing_closed_or_invalid_file() {
        let mut gc = Heap::new();
        let err = gc.write_host_file_byte(999, 65).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn should_return_error_when_reading_from_writer() {
        let mut gc = Heap::new();
        let path = std::env::temp_dir().join("test_write.txt");
        let id = gc.open_host_output_file(&path).unwrap();
        let err = gc.read_host_file_byte(id).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
        gc.close_host_file(id);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn should_return_error_when_writing_to_reader() {
        let mut gc = Heap::new();
        let path = std::env::temp_dir().join("test_read.txt");
        std::fs::write(&path, b"hello").unwrap();
        let id = gc.open_host_input_file(&path).unwrap();
        let err = gc.write_host_file_byte(id, 65).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
        gc.close_host_file(id);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn should_return_error_when_opening_non_existent_file() {
        let mut gc = Heap::new();
        let path = std::env::temp_dir().join("definitely_does_not_exist_1234.txt");
        let err = gc.open_host_input_file(&path).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/FileNotFoundException")
        );
    }

    #[test]
    fn should_return_error_when_spawning_empty_command() {
        let mut gc = Heap::new();
        let err = gc.spawn_host_process(&[], None).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn should_cache_process_exit_code() {
        let mut gc = Heap::new();
        let command = if cfg!(windows) {
            vec![
                "cmd".to_string(),
                "/C".to_string(),
                "echo hello".to_string(),
            ]
        } else {
            vec!["echo".to_string(), "hello".to_string()]
        };
        let process = gc.spawn_host_process(&command, None).unwrap();
        let code = gc.wait_host_process(process.process_id).unwrap();
        assert_eq!(code, 0);
        // Should use cache
        let code2 = gc.wait_host_process(process.process_id).unwrap();
        assert_eq!(code2, 0);
        // Try wait should use cache
        let code3 = gc.try_host_process_exit_value(process.process_id).unwrap();
        assert_eq!(code3, Some(0));
        // Destroy on already exited should be ok
        gc.destroy_host_process(process.process_id).unwrap();
    }

    #[test]
    fn should_return_error_when_waiting_invalid_process() {
        let mut gc = Heap::new();
        let err = gc.wait_host_process(999).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn should_return_error_when_destroying_invalid_process() {
        let mut gc = Heap::new();
        let err = gc.destroy_host_process(999).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn should_return_error_when_trying_exit_value_invalid_process() {
        let mut gc = Heap::new();
        let err = gc.try_host_process_exit_value(999).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }

    #[test]
    fn should_return_error_when_binding_invalid_socket_address() {
        let mut gc = Heap::new();
        let err = gc.bind_server_socket("invalid_address").unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/net/SocketException")
        );
    }

    #[test]
    fn should_return_error_when_accepting_invalid_listener() {
        let mut gc = Heap::new();
        let err = gc.accept_connection(999).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name == "java/io/IOException")
        );
    }
    use super::*;

    // ── Allocation ────────────────────────────────────────────────────────────

    #[test]
    fn allocate_and_get() {
        let mut heap = Heap::new();
        let r = heap.allocate("Point".to_string(), 2);
        assert_eq!(r & OLD_BIT, 0, "new object must be in young gen");
        let obj = heap.get(r).unwrap();
        assert_eq!(obj.class_name, "Point");
        assert_eq!(obj.fields.len(), 2);
        assert_eq!(obj.fields[0], Slot::Int(0));
    }

    #[test]
    fn allocate_multiple() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("Point".to_string(), 2);
        let r1 = heap.allocate("Point".to_string(), 2);
        assert_eq!(r0, 0);
        assert_eq!(r1, 1);
        assert_eq!(heap.len(), 2);
    }

    #[test]
    fn get_mut_sets_field() {
        let mut heap = Heap::new();
        let r = heap.allocate("Point".to_string(), 2);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(42);
        assert_eq!(heap.get(r).unwrap().fields[0], Slot::Int(42));
    }

    #[test]
    fn invalid_ref_returns_error() {
        let heap = Heap::new();
        let err = heap.get(999).unwrap_err();
        assert!(matches!(err, Error::InvalidRef { address: 999 }));
    }

    #[test]
    fn get_on_invalid_ref_returns_error() {
        let heap = Heap::new();
        assert!(heap.get(0).is_err());
        assert!(heap.get(999).is_err());
    }

    #[test]
    fn allocate_string_stores_value() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello".to_string());
        let obj = heap.get(r).unwrap();
        assert_eq!(obj.class_name, "java/lang/String");
        assert_eq!(obj.string_value, Some("hello".to_string()));
        // Real 4-slot layout: value:[B, coder:B, hash:I, hashIsZero:Z.
        assert_eq!(obj.fields.len(), 4);
        assert!(matches!(obj.fields[0], Slot::Reference(Some(_))));
    }

    #[test]
    fn heap_object_marked_defaults_false() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert!(!heap.get(r).unwrap().marked);
    }

    #[test]
    fn heap_object_age_defaults_zero() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert_eq!(heap.get(r).unwrap().age, 0);
    }

    #[test]
    fn heap_object_forward_defaults_none() {
        let mut heap = Heap::new();
        let r = heap.allocate("Foo".to_string(), 0);
        assert!(heap.get(r).unwrap().forward.is_none());
    }

    // ── GC triggers ───────────────────────────────────────────────────────────

    #[test]
    fn should_gc_triggers_at_2x_growth() {
        let mut heap = Heap::new();
        for i in 0..255 {
            heap.allocate(format!("C{i}"), 0);
            assert!(!heap.should_gc());
        }
        heap.allocate("C255".to_string(), 0);
        assert!(heap.should_gc());
    }

    // ── collect() compat shim ─────────────────────────────────────────────────

    #[test]
    fn collect_reclaims_unreachable() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("Keep".to_string(), 0);
        let _r1 = heap.allocate("Drop".to_string(), 0);
        let _r2 = heap.allocate("Drop".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(r0))]);
        assert_eq!(heap.free_list_len(), 2);
        assert_eq!(heap.len(), 1);
    }

    #[test]
    fn collect_preserves_reachable_chain() {
        let mut heap = Heap::new();
        let rc = heap.allocate("C".to_string(), 0);
        let rb = heap.allocate("B".to_string(), 1);
        heap.get_mut(rb).unwrap().fields[0] = Slot::Reference(Some(rc));
        let ra = heap.allocate("A".to_string(), 1);
        heap.get_mut(ra).unwrap().fields[0] = Slot::Reference(Some(rb));
        heap.collect(&[Slot::Reference(Some(ra))]);
        assert_eq!(heap.free_list_len(), 0);
        assert_eq!(heap.len(), 3);
    }

    #[test]
    fn free_list_slot_reused_after_collect() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("Keep".to_string(), 0);
        let _r1 = heap.allocate("Drop".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(r0))]);
        // After full collect, Keep was promoted to old gen.
        // Allocate a new object — it goes to young gen.
        let r2 = heap.allocate("New".to_string(), 0);
        // r2 should be a young-gen ref (OLD_BIT == 0).
        assert_eq!(r2 & OLD_BIT, 0);
        assert!(heap.get(r2).is_ok());
    }

    #[test]
    fn get_on_swept_slot_returns_error() {
        let mut heap = Heap::new();
        let keep = heap.allocate("Keep".to_string(), 0);
        let drop_r = heap.allocate("Drop".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(keep))]);
        // drop_r is now in the old-gen free list or absent.
        // Either way, get() on it must error.
        assert!(heap.get(drop_r).is_err() || heap.get(drop_r | OLD_BIT).is_err());
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn test_heap_with_capacity(cap: usize) -> Heap {
        let mut h = Heap::new();
        h.young_capacity = cap;
        h
    }

    fn make_old_obj(heap: &mut Heap) -> u64 {
        let obj = HeapObject {
            class_name: "OldObj".to_string(),
            fields: vec![Slot::Int(0)],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        };
        heap.old.push(Some(obj));
        (heap.old.len() as u64 - 1) | OLD_BIT
    }

    // ── GC trigger tests (Task 3) ──────────────────────────────────────────────

    #[test]
    fn should_minor_gc_fires_at_young_capacity() {
        let mut heap = Heap::new();
        heap.young_capacity = 4;
        // First 3 allocs should not trigger.
        for i in 0..3 {
            heap.allocate(format!("C{i}"), 0);
            assert!(
                !heap.should_minor_gc(),
                "should not fire before reaching capacity"
            );
        }
        // 4th alloc hits young_top == young_capacity → fires.
        heap.allocate("C3".to_string(), 0);
        assert!(heap.should_minor_gc());
    }

    #[test]
    fn should_major_gc_fires_at_2x_old_live() {
        let mut heap = Heap::new();
        // Simulate post-GC state: 10 live old objects, alloc_since_gc reset to 0.
        // Use collect() on a fresh heap to set live_after_last_gc.
        // First, make 10 objects live through a collect.
        let roots: Vec<Slot> = (0..10)
            .map(|_| {
                let r = heap.allocate("O".to_string(), 0);
                Slot::Reference(Some(r))
            })
            .collect();
        heap.collect(&roots);
        // Now live_after_last_gc == 10 (all 10 in old gen after promotion).
        // threshold = max(10*2, 256) = 256. Must allocate 256 more.
        for i in 0..255 {
            heap.allocate(format!("X{i}"), 0);
            assert!(!heap.should_major_gc(), "should not fire at alloc {i}");
        }
        heap.allocate("X255".to_string(), 0);
        assert!(heap.should_major_gc());
    }

    // ── write_field tests (Task 4) ─────────────────────────────────────────────

    #[test]
    fn write_field_old_to_young_adds_to_remembered_set() {
        let mut heap = Heap::new();
        let old_ref = make_old_obj(&mut heap);
        let young_ref = heap.allocate("Young".to_string(), 0);
        heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref)))
            .unwrap();
        let old_idx = (old_ref & !OLD_BIT) as usize;
        assert!(
            heap.remembered_set.contains(&old_idx),
            "old→young store must populate remembered_set"
        );
    }

    #[test]
    fn write_field_young_to_young_does_not_add_to_remembered_set() {
        let mut heap = Heap::new();
        let r0 = heap.allocate("A".to_string(), 1);
        let r1 = heap.allocate("B".to_string(), 0);
        // r0 is young; store another young ref into it.
        heap.write_field(r0, 0, Slot::Reference(Some(r1))).unwrap();
        assert!(
            heap.remembered_set.is_empty(),
            "young→young store must NOT populate remembered_set"
        );
    }

    #[test]
    fn write_field_old_to_old_does_not_add_to_remembered_set() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "A".to_string(),
            fields: vec![Slot::Int(0)],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "B".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let a_ref = OLD_BIT;
        let b_ref = 1u64 | OLD_BIT;
        heap.write_field(a_ref, 0, Slot::Reference(Some(b_ref)))
            .unwrap();
        assert!(
            heap.remembered_set.is_empty(),
            "old→old store must NOT populate remembered_set"
        );
    }

    #[test]
    fn write_field_out_of_bounds_returns_error_without_side_effects() {
        let mut heap = Heap::new();
        let old_ref = make_old_obj(&mut heap);
        let young_ref = heap.allocate("Young".to_string(), 0);

        let err = heap
            .write_field(old_ref, 1, Slot::Reference(Some(young_ref)))
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOutOfBounds {
                index: 1,
                length: 1
            }
        );
        assert_eq!(heap.get(old_ref).unwrap().fields[0], Slot::Int(0));
        assert!(
            heap.remembered_set.is_empty(),
            "failed writes must not populate remembered_set"
        );
    }

    #[test]
    fn write_field_implementation_does_not_unwrap_after_validation() {
        let code = std::fs::read_to_string("src/lib.rs").unwrap();
        let start = code.find("pub fn write_field").unwrap();
        let end = code[start..].find("pub fn minor_collect_prepare").unwrap();
        let function_body = &code[start..start + end];

        assert!(
            !function_body.contains("unwrap().fields"),
            "write_field should propagate accessor errors instead of unwrapping"
        );
    }

    #[test]
    fn heap_implementation_uses_checked_old_ref_conversion() {
        let code = std::fs::read_to_string("src/lib.rs").unwrap();
        let end = code.find("mod tests").unwrap();
        let implementation = &code[..end];

        assert!(
            !implementation.contains("& !OLD_BIT) as usize"),
            "old-gen reference decoding must not truncate through `as usize`"
        );
    }

    // ── Minor GC unit tests (Task 6) ───────────────────────────────────────────

    #[test]
    fn minor_gc_copies_reachable_young_object() {
        let mut heap = test_heap_with_capacity(8);
        let r0 = heap.allocate("Keep".to_string(), 0);
        let r1 = heap.allocate("Drop".to_string(), 0);
        let roots = vec![Slot::Reference(Some(r0))];
        heap.minor_collect_prepare(&roots);
        // r0 must have a forwarding pointer; r1 must not.
        assert!(
            heap.young[usize::try_from(r0).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .is_some()
        );
        assert!(
            heap.young[usize::try_from(r1).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .is_none()
        );
    }

    #[test]
    fn minor_gc_forward_patches_root_slot() {
        let mut heap = test_heap_with_capacity(8);
        let r0 = heap.allocate("A".to_string(), 0);
        let roots = vec![Slot::Reference(Some(r0))];
        heap.minor_collect_prepare(&roots);
        let mut slot = Slot::Reference(Some(r0));
        heap.apply_forward(&mut slot);
        // After forwarding, slot must point to the new location.
        let new_r = heap.young[usize::try_from(r0).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        assert_eq!(slot, Slot::Reference(Some(new_r)));
    }

    #[test]
    fn minor_gc_finish_swaps_to_space_into_young() {
        let mut heap = test_heap_with_capacity(8);
        let r0 = heap.allocate("A".to_string(), 0);
        let _r1 = heap.allocate("B".to_string(), 0);
        // Only r0 is a root → _r1 is dead.
        let roots = vec![Slot::Reference(Some(r0))];
        heap.minor_collect_prepare(&roots);
        heap.minor_collect_finish();
        // After finish: young has 1 live survivor.
        assert_eq!(heap.young.iter().filter(|s| s.is_some()).count(), 1);
        assert!(heap.to_space.is_empty());
        assert!(heap.remembered_set.is_empty());
    }

    #[test]
    fn minor_gc_increments_age_on_survival() {
        let mut heap = test_heap_with_capacity(8);
        let r = heap.allocate("Survivor".to_string(), 0);
        let roots = vec![Slot::Reference(Some(r))];
        heap.minor_collect_prepare(&roots);
        // Locate the copy in to_space (new_ref from forward pointer).
        let new_r = heap.young[usize::try_from(r).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        heap.minor_collect_finish();
        // After finish, young is former to_space. new_r has no OLD_BIT → young index.
        let survivor = heap.young[usize::try_from(new_r).unwrap()]
            .as_ref()
            .unwrap();
        assert_eq!(survivor.age, 1);
    }

    #[test]
    fn minor_gc_promotes_at_promotion_age() {
        let mut heap = test_heap_with_capacity(64);
        // promotion_age = 1: an object with age >= 1 is promoted.
        // Round 0: age=0, check 0>=1 → false → to_space (age becomes 1), new_r is young.
        // Round 1: age=1, check 1>=1 → true  → old gen (OLD_BIT set).
        heap.promotion_age = 1;

        let mut current_r = heap.allocate("P".to_string(), 0);

        for round in 0..2u8 {
            let roots = vec![Slot::Reference(Some(current_r))];
            heap.minor_collect_prepare(&roots);
            let new_r = heap.young[usize::try_from(current_r).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .unwrap();
            heap.minor_collect_finish();

            if round == 0 {
                // Still young after first survival (age becomes 1, not yet promoted).
                assert_eq!(new_r & OLD_BIT, 0, "should still be young after 1 survival");
                current_r = new_r;
            } else {
                // Promoted to old gen (OLD_BIT set) on second survival.
                assert_ne!(new_r & OLD_BIT, 0, "should be in old gen after 2 survivals");
                let obj = heap.get(new_r).unwrap();
                assert_eq!(obj.class_name, "P");
            }
        }
    }

    #[test]
    fn remembered_set_root_survives_minor_gc() {
        let mut heap = test_heap_with_capacity(8);
        // Build: old-gen object with a field pointing to a young object.
        heap.old.push(Some(HeapObject {
            class_name: "Old".to_string(),
            fields: vec![Slot::Int(0)], // will be overwritten below
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let old_ref = OLD_BIT;
        let young_ref = heap.allocate("Young".to_string(), 0);
        // Wire old→young via write_field (populates remembered_set).
        heap.write_field(old_ref, 0, Slot::Reference(Some(young_ref)))
            .unwrap();

        // No stack roots — young object reachable only through remembered set.
        heap.minor_collect_prepare(&[]);
        assert!(
            heap.young[usize::try_from(young_ref).unwrap()]
                .as_ref()
                .unwrap()
                .forward
                .is_some(),
            "young object reachable via rem-set must be forwarded"
        );
        heap.minor_collect_finish();
        // Verify old-gen field was patched to the new young location.
        let new_field = heap.old[0].as_ref().unwrap().fields[0];
        match new_field {
            Slot::Reference(Some(r)) => {
                heap.get(r).expect("patched old→young field must be valid");
            }
            other => panic!("expected Reference, got {other:?}"),
        }
    }

    #[test]
    fn apply_forward_is_no_op_on_non_references() {
        let heap = Heap::new();
        let mut slot = Slot::Int(42);
        heap.apply_forward(&mut slot);
        assert_eq!(slot, Slot::Int(42));
    }

    #[test]
    fn apply_forward_is_no_op_on_null_ref() {
        let heap = Heap::new();
        let mut slot = Slot::Reference(None);
        heap.apply_forward(&mut slot);
        assert_eq!(slot, Slot::Reference(None));
    }

    #[test]
    fn apply_forward_is_no_op_on_old_gen_ref() {
        let heap = Heap::new();
        let old_ref = OLD_BIT;
        let mut slot = Slot::Reference(Some(old_ref));
        heap.apply_forward(&mut slot);
        // No forwarding pointer in old gen → slot unchanged.
        assert_eq!(slot, Slot::Reference(Some(old_ref)));
    }

    // ── Major GC unit tests (Task 7) ───────────────────────────────────────────

    #[test]
    fn old_ref_has_old_bit() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "OldObj".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let old_ref = OLD_BIT;
        assert_ne!(old_ref & OLD_BIT, 0, "old ref must have OLD_BIT set");
        assert_eq!(heap.get(old_ref).unwrap().class_name, "OldObj");
    }

    #[test]
    fn major_collect_reclaims_unreachable_old_objects() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "Keep".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "Drop".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let keep_ref = OLD_BIT;
        let drop_ref = 1u64 | OLD_BIT;
        let roots = vec![Slot::Reference(Some(keep_ref))];
        heap.major_collect(&roots);
        assert!(
            heap.get(keep_ref).is_ok(),
            "reachable old-gen object must survive"
        );
        assert!(
            heap.get(drop_ref).is_err(),
            "unreachable old-gen object must be swept"
        );
        assert_eq!(heap.old_free_list.len(), 1);
    }

    #[test]
    fn major_collect_preserves_reachable_chain_in_old_gen() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "C".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "B".to_string(),
            fields: vec![Slot::Reference(Some(OLD_BIT))],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "A".to_string(),
            fields: vec![Slot::Reference(Some(1u64 | OLD_BIT))],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let a_ref = 2u64 | OLD_BIT;
        let roots = vec![Slot::Reference(Some(a_ref))];
        heap.major_collect(&roots);
        assert_eq!(heap.old_live_count(), 3);
        assert_eq!(heap.old_free_list.len(), 0);
    }

    #[test]
    fn major_collect_reuses_freed_slot() {
        let mut heap = Heap::new();
        heap.old.push(Some(HeapObject {
            class_name: "Keep".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "Drop".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let keep_ref = OLD_BIT;
        heap.major_collect(&[Slot::Reference(Some(keep_ref))]);
        // old_free_list has raw index 1 (no OLD_BIT).
        assert!(heap.old_free_list.contains(&1u64));
    }

    #[test]
    fn collect_compat_shim_collects_full_heap() {
        let mut heap = test_heap_with_capacity(512);
        // promotion_age = 0: age >= 0 is always true, so any survivor promotes
        // on the first minor GC.  This lets the compat collect() shim produce a
        // single old-gen object in one pass.
        heap.promotion_age = 0;
        let keep = heap.allocate("Keep".to_string(), 0);
        let _drop1 = heap.allocate("Drop1".to_string(), 0);
        let _drop2 = heap.allocate("Drop2".to_string(), 0);
        let roots = vec![Slot::Reference(Some(keep))];
        heap.collect(&roots);
        // After full collect: 1 live object promoted to old gen; 2 dropped.
        assert_eq!(heap.old_live_count(), 1);
        assert_eq!(heap.len(), 1);
    }

    // ── allocate_string coverage ───────────────────────────────────────────────

    #[test]
    fn allocate_string_increments_live_count() {
        let mut heap = Heap::new();
        assert_eq!(heap.len(), 0);
        // Each allocate_string mints two objects: the String and its backing [B.
        heap.allocate_string("a".to_string());
        assert_eq!(heap.len(), 2);
        heap.allocate_string("b".to_string());
        assert_eq!(heap.len(), 4);
    }

    #[test]
    fn allocate_string_returns_distinct_refs() {
        let mut heap = Heap::new();
        let r0 = heap.allocate_string("hello".to_string());
        let r1 = heap.allocate_string("world".to_string());
        assert_ne!(r0, r1);
        assert_eq!(
            heap.get(r0).unwrap().string_value,
            Some("hello".to_string())
        );
        assert_eq!(
            heap.get(r1).unwrap().string_value,
            Some("world".to_string())
        );
    }

    #[test]
    fn find_string_backed_object_matches_class_and_payload() {
        let mut heap = Heap::new();
        let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
        heap.get_mut(class_ref).unwrap().string_value = Some("java/lang/String".to_string());
        let other_ref = heap.allocate_string("java/lang/String".to_string());

        assert_eq!(
            heap.find_string_backed_object("java/lang/Class", "java/lang/String"),
            Some(class_ref)
        );
        assert_eq!(
            heap.find_string_backed_object("java/lang/String", "java/lang/String"),
            Some(other_ref)
        );
        assert_eq!(
            heap.find_string_backed_object("java/lang/Class", "java/lang/Object"),
            None
        );
    }

    #[test]
    fn allocate_string_increments_alloc_since_gc() {
        let mut heap = Heap::new();
        // Each allocate_string bumps alloc_since_gc twice (String + backing [B),
        // so the 256-allocation threshold is reached after 128 calls.
        for i in 0..127 {
            heap.allocate_string(format!("s{i}"));
            assert!(!heap.should_gc());
        }
        heap.allocate_string("s127".to_string());
        assert!(heap.should_gc());
    }

    // ── real String layout (value:[B / coder:B) ───────────────────────────────

    #[test]
    fn allocate_string_latin1_layout_and_coder() {
        let mut heap = Heap::new();
        let s = heap.allocate_string("abc".to_string());
        let obj = heap.get(s).unwrap();
        assert_eq!(obj.fields.len(), 4);
        // coder = 0 (Latin-1) when every char is <= 0xFF.
        assert_eq!(obj.fields[1], Slot::Int(0), "Latin-1 coder must be 0");
        // hash / hashIsZero start at 0.
        assert_eq!(obj.fields[2], Slot::Int(0));
        assert_eq!(obj.fields[3], Slot::Int(0));
        // value:[B — a reference to a 3-byte array (1 byte/char for Latin-1).
        let bytes_ref = obj.fields[0]
            .as_reference()
            .expect("value slot must be a [B reference");
        let bytes = heap.get(bytes_ref).unwrap();
        assert_eq!(bytes.class_name, "[B");
        assert_eq!(bytes.fields.len(), 3);
        assert_eq!(bytes.fields[0], Slot::Int(i32::from(b'a')));
        assert_eq!(bytes.fields[1], Slot::Int(i32::from(b'b')));
        assert_eq!(bytes.fields[2], Slot::Int(i32::from(b'c')));
    }

    #[test]
    fn allocate_string_utf16_layout_and_coder() {
        let mut heap = Heap::new();
        // U+4E2D (中) is a non-Latin-1 BMP char → UTF-16 coder, 2 bytes/unit.
        let value = "a中b";
        let s = heap.allocate_string(value.to_string());
        let obj = heap.get(s).unwrap();
        assert_eq!(
            obj.fields[1],
            Slot::Int(1),
            "non-Latin-1 content must use coder 1 (UTF-16)"
        );
        let bytes_ref = obj.fields[0].as_reference().unwrap();
        let bytes = heap.get(bytes_ref).unwrap();
        // 3 UTF-16 code units × 2 bytes = 6 bytes.
        assert_eq!(bytes.fields.len(), value.encode_utf16().count() * 2);
        assert_eq!(bytes.fields.len(), 6);
    }

    #[test]
    fn allocate_string_stores_signed_java_bytes() {
        // A Latin-1 char > 0x7F is stored as a NEGATIVE Java byte (signed).
        let mut heap = Heap::new();
        let s = heap.allocate_string("\u{00E9}".to_string()); // é = 0xE9
        let obj = heap.get(s).unwrap();
        assert_eq!(obj.fields[1], Slot::Int(0), "é is Latin-1 → coder 0");
        let bytes_ref = obj.fields[0].as_reference().unwrap();
        let bytes = heap.get(bytes_ref).unwrap();
        assert_eq!(bytes.fields.len(), 1);
        assert_eq!(
            bytes.fields[0],
            Slot::Int(i32::from(i8::from_ne_bytes([0xE9])))
        );
        assert_eq!(bytes.fields[0], Slot::Int(-23));
    }

    #[test]
    fn string_layout_survives_gc_promotion_with_stable_identity() {
        let mut heap = Heap::new();
        heap.promotion_age = 0; // promote survivors to old on the first collect
        let value = "héllo中"; // mixed BMP → UTF-16
        let s = heap.allocate_string(value.to_string());
        let hash_before = heap.identity_hash(s).unwrap();

        // Full collect: minor GC promotes the String AND its backing [B (reachable
        // only through slot0) to old gen, rewriting slot0 to the promoted [B.
        heap.collect(&[Slot::Reference(Some(s))]);
        let moved = remap(&heap, s);
        assert_ne!(
            moved & OLD_BIT,
            0,
            "String must have been promoted to old gen"
        );

        let obj = heap.get(moved).unwrap();
        // string_value side-channel rode along on the object move.
        assert_eq!(obj.string_value.as_deref(), Some(value));
        // coder unchanged; slot0 [B was traced + forwarded to a live object.
        assert_eq!(obj.fields[1], Slot::Int(1));
        let bytes_ref = obj.fields[0]
            .as_reference()
            .expect("value [B must survive GC");
        let bytes = heap.get(bytes_ref).unwrap();
        assert_eq!(bytes.class_name, "[B");
        assert_eq!(bytes.fields.len(), value.encode_utf16().count() * 2);
        // Identity hash is stable across the move.
        assert_eq!(heap.identity_hash(moved).unwrap(), hash_before);
    }

    #[test]
    fn string_value_byte_array_edge_rewritten_by_old_compaction() {
        // Old gen: [garbage@0, [B@1, String@2]. Freeing the garbage hole makes the
        // [B slide down during compaction, so the String's slot0 value:[B edge must
        // be rewritten to the [B's new location (#1312 forwarding through the new slot).
        let mut heap = Heap::new();
        let garbage = push_old(&mut heap, "Garbage", vec![]);
        let bytes = push_old(
            &mut heap,
            "[B",
            vec![Slot::Int(i32::from(b'h')), Slot::Int(i32::from(b'i'))],
        );
        let string = push_old(
            &mut heap,
            "java/lang/String",
            vec![
                Slot::Reference(Some(bytes)),
                Slot::Int(0),
                Slot::Int(0),
                Slot::Int(0),
            ],
        );
        heap.get_mut(string).unwrap().string_value = Some("hi".to_string());
        let _ = garbage;

        // Root only the String; the garbage hole is unrooted → freed → the [B slides.
        heap.compact_old(&[Slot::Reference(Some(string))]);

        let string_after = remap(&heap, string);
        let bytes_after = remap(&heap, bytes);
        assert_ne!(
            bytes_after, bytes,
            "the backing [B must have slid during compaction"
        );
        // The String's slot0 edge was rewritten to the [B's new address.
        let slot0 = heap.get(string_after).unwrap().fields[0]
            .as_reference()
            .unwrap();
        assert_eq!(
            slot0, bytes_after,
            "String value:[B edge must be rewritten by compaction"
        );
        // Content intact after the move.
        let arr = heap.get(bytes_after).unwrap();
        assert_eq!(arr.fields.len(), 2);
        assert_eq!(
            heap.get(string_after).unwrap().string_value.as_deref(),
            Some("hi")
        );
    }

    // ── promote_to_old free-list path ─────────────────────────────────────────

    #[test]
    fn promote_to_old_free_list_path_sets_old_bit() {
        let mut heap = test_heap_with_capacity(64);
        heap.promotion_age = 0; // promote on first minor GC

        // Round 1: promote an object via push path → old[0]
        let r0 = heap.allocate("TempOld".to_string(), 0);
        heap.minor_collect_prepare(&[Slot::Reference(Some(r0))]);
        heap.minor_collect_finish();
        // old[0] holds TempOld; major GC with no roots frees it → raw idx 0 → free list
        heap.major_collect(&[]);
        assert!(
            !heap.old_free_list.is_empty(),
            "free list must be non-empty after sweep"
        );

        // Round 2: promote via free-list path (raw_idx=0 from old_free_list)
        let r1 = heap.allocate("NewObj".to_string(), 0);
        heap.minor_collect_prepare(&[Slot::Reference(Some(r1))]);
        let new_r1 = heap.young[usize::try_from(r1).unwrap()]
            .as_ref()
            .unwrap()
            .forward
            .unwrap();
        heap.minor_collect_finish();

        assert_ne!(
            new_r1 & OLD_BIT,
            0,
            "promoted object (via free-list) must have OLD_BIT set"
        );
        assert!(heap.get(new_r1).is_ok());
        assert_eq!(heap.get(new_r1).unwrap().class_name, "NewObj");
    }

    // ── is_empty ──────────────────────────────────────────────────────────────

    #[test]
    fn is_empty_on_fresh_heap() {
        let heap = Heap::new();
        assert!(heap.is_empty());
    }

    #[test]
    fn is_empty_false_after_allocation() {
        let mut heap = Heap::new();
        heap.allocate("X".to_string(), 0);
        assert!(!heap.is_empty());
    }

    // ── should_major_gc 2× multiplier ────────────────────────────────────────

    #[test]
    fn should_major_gc_uses_2x_threshold_not_add_or_div() {
        let mut heap = test_heap_with_capacity(512);
        heap.promotion_age = 0;
        // Populate old gen with 200 live objects via collect → live_after_last_gc=200.
        let roots: Vec<Slot> = (0..200)
            .map(|_| Slot::Reference(Some(heap.allocate("O".to_string(), 0))))
            .collect();
        heap.collect(&roots);
        // threshold = max(200*2, 256) = 400. Allocate 300 → should NOT fire.
        for _ in 0..300 {
            heap.allocate("X".to_string(), 0);
        }
        assert!(
            !heap.should_major_gc(),
            "should not fire at 300 allocs (threshold 400 with 200 live)"
        );
        // Allocate 100 more → alloc_since_gc=400 ≥ threshold=400 → should fire.
        for _ in 0..100 {
            heap.allocate("X".to_string(), 0);
        }
        assert!(
            heap.should_major_gc(),
            "should fire at 400 allocs (threshold 400 with 200 live)"
        );
    }

    // ── has_pending_forwards ──────────────────────────────────────────────────

    #[test]
    fn has_pending_forwards_false_before_gc() {
        let heap = Heap::new();
        assert!(!heap.has_pending_forwards());
    }

    #[test]
    fn has_pending_forwards_true_after_prepare() {
        let mut heap = test_heap_with_capacity(8);
        let r = heap.allocate("A".to_string(), 0);
        heap.minor_collect_prepare(&[Slot::Reference(Some(r))]);
        assert!(heap.has_pending_forwards());
    }

    // ── minor GC patches old-gen fields when survivor index changes ──────────

    #[test]
    fn minor_gc_patches_old_gen_field_when_young_index_changes() {
        let mut heap = test_heap_with_capacity(8);
        // Allocate two young objects: young[0] will die, young[1] will survive.
        let dead = heap.allocate("Dead".to_string(), 0);
        let survive = heap.allocate("Survive".to_string(), 0);
        assert_eq!(dead, 0);
        assert_eq!(survive, 1, "survive must be at young index 1 for this test");

        // Create old object with field pointing to young[1]; write_field populates remembered_set.
        let old_ref = make_old_obj(&mut heap); // old[0], fields[0] = Int(0)
        heap.write_field(old_ref, 0, Slot::Reference(Some(survive)))
            .unwrap();

        // Minor GC: no stack roots. young[1] survives via remembered set → forwarded to to_space[0].
        heap.minor_collect_prepare(&[]);
        heap.minor_collect_finish();

        // After GC: young = to_space = [Some(Survive)]. "Survive" is now at index 0.
        // Old field must be patched from 1 → 0.
        let field = heap.get(old_ref).unwrap().fields[0];
        match field {
            Slot::Reference(Some(r)) => {
                assert_eq!(r & OLD_BIT, 0, "patched ref must be young");
                assert!(
                    heap.get(r).is_ok(),
                    "patched old→young field must be accessible"
                );
                assert_eq!(heap.get(r).unwrap().class_name, "Survive");
            }
            other => panic!("expected Reference, got {other:?}"),
        }
    }

    #[test]
    fn minor_gc_patches_young_survivor_field_when_child_index_changes() {
        let mut heap = test_heap_with_capacity(8);
        let _dead = heap.allocate("Dead".to_string(), 0);
        let parent = heap.allocate("Parent".to_string(), 1);
        let child = heap.allocate("Child".to_string(), 0);
        heap.write_field(parent, 0, Slot::Reference(Some(child)))
            .unwrap();

        heap.minor_collect_prepare(&[Slot::Reference(Some(parent))]);
        let mut parent_slot = Slot::Reference(Some(parent));
        heap.apply_forward(&mut parent_slot);
        let Slot::Reference(Some(new_parent)) = parent_slot else {
            panic!("expected forwarded parent ref");
        };
        heap.minor_collect_finish();

        let Slot::Reference(Some(patched_child)) = heap.get(new_parent).unwrap().fields[0] else {
            panic!("expected forwarded child ref");
        };
        assert_eq!(
            patched_child, 1,
            "child should move from young[2] to young[1]"
        );
        assert_eq!(heap.get(patched_child).unwrap().class_name, "Child");
    }

    #[test]
    fn promoted_old_object_keeps_remembered_set_for_next_minor_gc() {
        let mut heap = test_heap_with_capacity(8);
        heap.promotion_age = 0;

        let parent = heap.allocate("Parent".to_string(), 1);
        let child = heap.allocate("Child".to_string(), 0);
        heap.write_field(parent, 0, Slot::Reference(Some(child)))
            .unwrap();

        heap.minor_collect_prepare(&[Slot::Reference(Some(parent))]);
        let mut parent_slot = Slot::Reference(Some(parent));
        heap.apply_forward(&mut parent_slot);
        let Slot::Reference(Some(promoted_parent)) = parent_slot else {
            panic!("expected promoted parent ref");
        };
        assert_ne!(
            promoted_parent & OLD_BIT,
            0,
            "parent should promote on first survival"
        );
        heap.minor_collect_finish();

        let _dead = heap.allocate("Dead".to_string(), 0);
        heap.minor_collect_prepare(&[]);
        heap.minor_collect_finish();

        let Slot::Reference(Some(still_live_child)) = heap.get(promoted_parent).unwrap().fields[0]
        else {
            panic!("expected promoted parent to keep child ref");
        };
        assert_eq!(heap.get(still_live_child).unwrap().class_name, "Child");
    }

    // ── minor_collect_finish live_count ───────────────────────────────────────

    #[test]
    fn minor_collect_finish_live_count_sums_generations() {
        let mut heap = test_heap_with_capacity(8);
        // One old object (directly pushed, bypasses live_count).
        let _old_ref = make_old_obj(&mut heap);
        // One young object that survives the minor GC.
        let young_r = heap.allocate("Y".to_string(), 0);
        heap.minor_collect_prepare(&[Slot::Reference(Some(young_r))]);
        heap.minor_collect_finish();
        // minor_collect_finish recalculates live_count = young_live + old_live = 1 + 1 = 2.
        assert_eq!(heap.len(), 2);
    }

    // ── major_collect: young refs in roots/children must be ignored ──────────

    #[test]
    fn major_gc_ignores_young_refs_in_roots() {
        let mut heap = Heap::new();
        // Two old objects: old[0] should die, old[1] should survive.
        heap.old.push(Some(HeapObject {
            class_name: "ShouldDie".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        heap.old.push(Some(HeapObject {
            class_name: "ShouldLive".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let die_ref = OLD_BIT;
        let live_ref = 1u64 | OLD_BIT;
        // Young object at index 0 — same raw index as old[0].
        let young_r = heap.allocate("Young".to_string(), 0);
        assert_eq!(young_r, 0);
        // Roots: live_ref (old) + young_r (young). Young ref must NOT mark old[0].
        heap.major_collect(&[
            Slot::Reference(Some(live_ref)),
            Slot::Reference(Some(young_r)),
        ]);
        assert!(
            heap.get(die_ref).is_err(),
            "unreachable old object must be swept even when young ref shares raw index"
        );
        assert!(
            heap.get(live_ref).is_ok(),
            "reachable old object must survive"
        );
    }

    #[test]
    fn major_gc_does_not_follow_young_refs_as_old_gen_children() {
        let mut heap = Heap::new();
        // old[0] = "ShouldDie" (unreachable); raw idx 0 matches young[0].
        heap.old.push(Some(HeapObject {
            class_name: "ShouldDie".to_string(),
            fields: vec![],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        // Allocate young[0] so raw idx 0 exists in young gen too.
        let young_r = heap.allocate("Young".to_string(), 0);
        assert_eq!(young_r, 0);
        // old[1] = "Parent" with a field pointing to young[0].
        heap.old.push(Some(HeapObject {
            class_name: "Parent".to_string(),
            fields: vec![Slot::Reference(Some(young_r))],
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        let die_ref = OLD_BIT;
        let parent_ref = 1u64 | OLD_BIT;
        // Root is only "Parent". The young ref in Parent's fields must not mark old[0].
        heap.major_collect(&[Slot::Reference(Some(parent_ref))]);
        assert!(
            heap.get(die_ref).is_err(),
            "old[0] must be swept: the young ref in Parent's fields must not mark it"
        );
        assert!(heap.get(parent_ref).is_ok(), "Parent must survive");
    }

    // ── Performance Tests ──────────────────────────────────────────────────────

    #[test]
    fn minor_gc_avoids_intermediate_allocs() {
        let code = std::fs::read_to_string("src/lib.rs").unwrap();
        // find the start of the minor_collect_prepare function
        let start = code.find("pub fn minor_collect_prepare").unwrap();
        let end = code[start..].find("pub fn minor_collect_finish").unwrap();
        let function_body = &code[start..start + end];

        assert!(
            !function_body.contains("let young_refs: Vec<usize> ="),
            "minor_collect_prepare should not collect into an intermediate vector for young_refs"
        );
        assert!(
            !function_body.contains("let children: Vec<usize> ="),
            "minor_collect_prepare should not collect into an intermediate vector for children"
        );
    }

    // ── TCP socket tests (Task 1) ──────────────────────────────────────────────

    #[test]
    fn bind_server_socket_returns_valid_id() {
        let mut heap = Heap::new();
        let id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
        assert!(id > 0);
    }

    #[test]
    fn bind_server_socket_addr_in_use() {
        let mut heap = Heap::new();
        let id = heap
            .bind_server_socket("127.0.0.1:0")
            .expect("first bind failed");
        let port = heap.server_socket_local_port(id).expect("port failed");
        let err = heap
            .bind_server_socket(&format!("127.0.0.1:{port}"))
            .unwrap_err();
        assert!(matches!(
            err,
            duke_runtime::Error::JavaException { ref class_name }
            if class_name == "java/net/BindException"
        ));
    }

    #[test]
    fn connect_socket_refused() {
        // On Windows, WSAECONNREFUSED may not map to ErrorKind::ConnectionRefused in all
        // Rust versions. Accept either ConnectException or SocketException so the test
        // passes on all platforms while still verifying no panic occurs.
        // Bind to get a port, then drop the listener so nothing listens.
        let mut heap = Heap::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let err = heap
            .connect_socket(&format!("127.0.0.1:{port}"))
            .unwrap_err();
        assert!(matches!(
            err,
            duke_runtime::Error::JavaException { ref class_name }
            if class_name == "java/net/ConnectException"
                || class_name == "java/net/SocketException"
        ));
    }

    #[test]
    fn server_socket_local_port() {
        let mut heap = Heap::new();
        let id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
        let port = heap.server_socket_local_port(id).expect("port failed");
        assert!(port > 0);
    }

    #[test]
    fn accept_and_read_roundtrip() {
        let mut heap = Heap::new();
        let server_id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
        let port = heap
            .server_socket_local_port(server_id)
            .expect("port failed");
        let handle = std::thread::spawn(move || {
            let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            std::io::Write::write_all(&mut stream, &[42]).unwrap();
        });
        let (reader_id, _writer_id) = heap.accept_connection(server_id).expect("accept failed");
        let byte = heap.read_host_file_byte(reader_id).expect("read failed");
        assert_eq!(byte, 42);
        handle.join().unwrap();
    }

    #[test]
    fn connect_and_write_roundtrip() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1];
            std::io::Read::read_exact(&mut stream, &mut buf).unwrap();
            assert_eq!(buf[0], 99);
        });
        let mut heap = Heap::new();
        let (_reader_id, writer_id) = heap
            .connect_socket(&format!("127.0.0.1:{port}"))
            .expect("connect failed");
        heap.write_host_file_byte(writer_id, 99)
            .expect("write failed");
        // Drop the writer so the listener's read_exact completes.
        heap.close_host_file(writer_id);
        handle.join().unwrap();
    }

    #[test]
    fn close_then_read_returns_io_exception() {
        let mut heap = Heap::new();
        let server_id = heap.bind_server_socket("127.0.0.1:0").expect("bind failed");
        let port = heap
            .server_socket_local_port(server_id)
            .expect("port failed");
        let handle = std::thread::spawn(move || {
            let _stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        });
        let (reader_id, _writer_id) = heap.accept_connection(server_id).expect("accept failed");
        handle.join().unwrap();
        heap.close_host_file(reader_id);
        let err = heap.read_host_file_byte(reader_id).unwrap_err();
        assert!(matches!(
            err,
            duke_runtime::Error::JavaException { ref class_name }
            if class_name == "java/io/IOException"
        ));
    }

    // ── Phase 1: promotion / stable identity hash / executor-queue rooting ─────

    /// Drive a complete minor GC cycle (prepare → forward roots → finish) and
    /// return the forwarded copy of `roots`.
    fn run_minor_gc(heap: &mut Heap, roots: &[Slot]) -> Vec<Slot> {
        heap.minor_collect_prepare(roots);
        let mut patched: Vec<Slot> = roots.to_vec();
        for slot in &mut patched {
            heap.apply_forward(slot);
        }
        heap.minor_collect_finish();
        patched
    }

    /// Allocation churn: a flood of unreachable young objects is fully reclaimed
    /// by successive minor GCs, so the young generation stays bounded.
    #[test]
    fn allocation_churn_keeps_young_bounded() {
        let mut heap = test_heap_with_capacity(16);
        // One long-lived survivor we keep rooted throughout.
        let survivor = heap.allocate("Survivor".to_string(), 0);
        let mut survivor_slot = Slot::Reference(Some(survivor));

        for _ in 0..50 {
            // Churn: 100 short-lived objects nobody roots.
            for _ in 0..100 {
                let _garbage = heap.allocate("Garbage".to_string(), 1);
            }
            let patched = run_minor_gc(&mut heap, &[survivor_slot]);
            survivor_slot = patched[0];
            // After each collection only the single rooted survivor may remain young.
            assert!(
                heap.young.len() <= 1,
                "young gen must stay bounded, was {}",
                heap.young.len()
            );
        }
        // The survivor is still reachable and intact.
        assert_eq!(
            heap.get(survivor_slot.as_reference().unwrap())
                .unwrap()
                .class_name,
            "Survivor"
        );
    }

    /// A survivor kept rooted across enough minor GCs is evacuated into the old
    /// gen with its field and string contents intact and its root ref rewritten.
    #[test]
    fn survivor_promotion_preserves_contents_and_rewrites_root() {
        let mut heap = test_heap_with_capacity(32);
        // Default promotion age (4): promoted on the 5th survival.
        let payload = heap.allocate("Payload".to_string(), 2);
        let text = heap.allocate_string("hello gc".to_string());
        heap.write_field(payload, 0, Slot::Int(0x1234_5678))
            .unwrap();
        heap.write_field(payload, 1, Slot::Reference(Some(text)))
            .unwrap();

        let mut root = Slot::Reference(Some(payload));
        for _ in 0..=DEFAULT_PROMOTION_AGE {
            root = run_minor_gc(&mut heap, &[root])[0];
        }

        let promoted = root.as_reference().unwrap();
        assert_ne!(
            promoted & OLD_BIT,
            0,
            "survivor should be promoted to old gen"
        );

        let obj = heap.get(promoted).unwrap();
        assert_eq!(obj.class_name, "Payload");
        assert_eq!(obj.fields[0], Slot::Int(0x1234_5678));
        // The child string ref must have been rewritten to its new location too.
        let text_ref = obj.fields[1].as_reference().unwrap();
        assert_eq!(
            heap.get(text_ref).unwrap().string_value.as_deref(),
            Some("hello gc")
        );
    }

    /// Root remapping: a held root ref ends up pointing at the old-gen copy after
    /// promotion and reads back the same data written before the move.
    #[test]
    fn root_ref_remaps_to_old_copy_after_promotion() {
        let mut heap = test_heap_with_capacity(16);
        heap.promotion_age = 0; // promote on first survival
        let r = heap.allocate("Box".to_string(), 1);
        heap.write_field(r, 0, Slot::Int(99)).unwrap();

        let patched = run_minor_gc(&mut heap, &[Slot::Reference(Some(r))]);
        let new_ref = patched[0].as_reference().unwrap();

        assert_ne!(
            new_ref & OLD_BIT,
            0,
            "root should now name the old-gen copy"
        );
        assert_ne!(new_ref, r, "root ref must have been remapped");
        assert_eq!(heap.get(new_ref).unwrap().fields[0], Slot::Int(99));
    }

    /// Identity hash is stable across a minor GC that promotes the object.
    #[test]
    fn identity_hash_is_stable_across_promotion() {
        let mut heap = test_heap_with_capacity(16);
        heap.promotion_age = 0; // promote on first survival
        let r = heap.allocate("Ident".to_string(), 0);

        let before = heap.identity_hash(r).unwrap();

        let patched = run_minor_gc(&mut heap, &[Slot::Reference(Some(r))]);
        let new_ref = patched[0].as_reference().unwrap();
        assert_ne!(new_ref & OLD_BIT, 0, "object should have been promoted");
        assert_ne!(new_ref, r, "reference index changed across relocation");

        let after = heap.identity_hash(new_ref).unwrap();
        assert_eq!(
            before, after,
            "identity hash must survive relocation unchanged"
        );
    }

    /// Distinct objects get distinct identity hashes, and repeated queries are
    /// idempotent (lazily assigned once).
    #[test]
    fn identity_hash_is_distinct_and_idempotent() {
        let mut heap = Heap::new();
        let a = heap.allocate("A".to_string(), 0);
        let b = heap.allocate("B".to_string(), 0);
        let ha1 = heap.identity_hash(a).unwrap();
        let ha2 = heap.identity_hash(a).unwrap();
        let hb = heap.identity_hash(b).unwrap();
        assert_eq!(ha1, ha2, "repeated identity_hash must be stable");
        assert_ne!(
            ha1, hb,
            "distinct objects must get distinct identity hashes"
        );
    }

    /// After a promoted object gains an old→young edge, the remembered set keeps
    /// the young referent alive through the following minor GC.
    #[test]
    fn old_to_young_remembered_set_after_promotion() {
        let mut heap = test_heap_with_capacity(16);
        heap.promotion_age = 0; // promote on first survival

        // Promote the parent first (no children yet, so it lands in old gen).
        let parent = heap.allocate("Parent".to_string(), 1);
        let promoted_parent = run_minor_gc(&mut heap, &[Slot::Reference(Some(parent))])[0]
            .as_reference()
            .unwrap();
        assert_ne!(promoted_parent & OLD_BIT, 0, "parent should be in old gen");

        // Now create a fresh young child and store it into the old parent. This
        // is the old→young store the write barrier must remember.
        let child = heap.allocate("Child".to_string(), 0);
        heap.write_field(promoted_parent, 0, Slot::Reference(Some(child)))
            .unwrap();
        assert!(
            heap.remembered_set
                .contains(&old_raw_index(promoted_parent)),
            "old parent with a young child must be in the remembered set"
        );

        // Second GC roots nothing directly: child survives only via the parent's
        // remembered-set entry, and the parent's field is rewritten to the copy.
        run_minor_gc(&mut heap, &[]);
        let child_final = heap.get(promoted_parent).unwrap().fields[0]
            .as_reference()
            .unwrap();
        assert_eq!(heap.get(child_final).unwrap().class_name, "Child");
    }

    /// Helper: raw old-gen index for a remembered-set membership check.
    fn old_raw_index(r: u64) -> usize {
        usize::try_from(r & !OLD_BIT).unwrap()
    }

    /// Build an executor heap object whose queue holds a single task pointing at
    /// `future_ref`/`task_ref`. Returns the executor object ref.
    fn make_executor_with_task(heap: &mut Heap, future_ref: u64, task_ref: u64) -> u64 {
        let exec_ref = heap.allocate("Executor".to_string(), 0);
        heap.get_mut(exec_ref).unwrap().atomic_payload = Some(AtomicPayload::executor(1));
        let Some(AtomicPayload::Executor(shared)) = &heap.get(exec_ref).unwrap().atomic_payload
        else {
            panic!("expected executor payload");
        };
        shared.state.lock().unwrap().queue.push_back(ExecutorTask {
            future_ref,
            task_ref,
            kind: ExecutorTaskKind::Runnable,
        });
        exec_ref
    }

    /// Reads the single queued task from an executor object.
    fn executor_task(heap: &Heap, exec_ref: u64) -> ExecutorTask {
        let Some(AtomicPayload::Executor(shared)) = &heap.get(exec_ref).unwrap().atomic_payload
        else {
            panic!("expected executor payload");
        };
        *shared.state.lock().unwrap().queue.front().unwrap()
    }

    /// A queued task keeps its referents alive through a minor GC even when they
    /// are not otherwise reachable, and the bare queue refs are forwarded.
    #[test]
    fn executor_queue_roots_and_forwards_task_refs() {
        let mut heap = test_heap_with_capacity(16);
        heap.promotion_age = 0; // force promotion so the refs actually move

        let future = heap.allocate("Future".to_string(), 0);
        let task = heap.allocate("Runnable".to_string(), 0);
        // Only the executor object is rooted; the task/future survive solely via
        // the executor's queue.
        let exec = make_executor_with_task(&mut heap, future, task);

        let exec_after = run_minor_gc(&mut heap, &[Slot::Reference(Some(exec))])[0]
            .as_reference()
            .unwrap();

        let queued = executor_task(&heap, exec_after);
        // The bare queue refs were rewritten to the survivors' new locations…
        assert_ne!(queued.future_ref, future, "future_ref must be forwarded");
        assert_ne!(queued.task_ref, task, "task_ref must be forwarded");
        // …and still resolve to live objects of the right class.
        assert_eq!(heap.get(queued.future_ref).unwrap().class_name, "Future");
        assert_eq!(heap.get(queued.task_ref).unwrap().class_name, "Runnable");
    }

    /// The queue is rooted even when the executor object itself is unreachable:
    /// worker threads hold the shared state independently, so queued tasks must
    /// not be collected.
    #[test]
    fn executor_queue_survives_unreachable_executor_object() {
        let mut heap = test_heap_with_capacity(16);
        let future = heap.allocate("Future".to_string(), 0);
        let task = heap.allocate("Runnable".to_string(), 0);
        let exec = make_executor_with_task(&mut heap, future, task);

        // Keep an independent Arc to the shared state, mirroring a worker thread.
        let shared_arc = match &heap.get(exec).unwrap().atomic_payload {
            Some(AtomicPayload::Executor(shared)) => Arc::clone(shared),
            _ => panic!("expected executor payload"),
        };

        // Nothing is rooted — the executor object is unreachable this cycle.
        run_minor_gc(&mut heap, &[]);

        // The queued task refs were still forwarded to live survivors.
        let queued = *shared_arc.state.lock().unwrap().queue.front().unwrap();
        assert_eq!(heap.get(queued.future_ref).unwrap().class_name, "Future");
        assert_eq!(heap.get(queued.task_ref).unwrap().class_name, "Runnable");
    }

    // ── Old-gen mark-compact (Phase 2) ────────────────────────────────────────

    /// Push an object straight into the old gen for compaction tests, returning
    /// its OLD_BIT-tagged reference.
    fn push_old(heap: &mut Heap, class: &str, fields: Vec<Slot>) -> u64 {
        heap.old.push(Some(HeapObject {
            class_name: class.to_string(),
            fields,
            string_value: None,
            atomic_payload: None,
            marked: false,
            age: 0,
            forward: None,
            identity_hash: None,
        }));
        (heap.old.len() as u64 - 1) | OLD_BIT
    }

    /// Remap a single ref through the heap's forwarding map (what the interpreter
    /// does to every root after a collection).
    fn remap(heap: &Heap, r: u64) -> u64 {
        let mut slot = Slot::Reference(Some(r));
        heap.apply_forward(&mut slot);
        slot.as_reference().unwrap()
    }

    /// Fragmenting workload: 100 old objects, keep every 4th, compact. Asserts
    /// the free list fragments past threshold, then compaction preserves every
    /// survivor's data, shrinks the backing store to the live count, drops
    /// fragmentation to zero, and leaves a single dense trailing region that
    /// fresh promotions bump into.
    #[test]
    fn old_gen_compaction_reclaims_and_preserves_survivors() {
        let mut heap = Heap::new();
        let refs: Vec<u64> = (0..100)
            .map(|i| push_old(&mut heap, "Old", vec![Slot::Int(i)]))
            .collect();

        // Keep every 4th object (25 survivors); the rest become free-list holes.
        let keep_roots: Vec<Slot> = refs
            .iter()
            .step_by(4)
            .map(|&r| Slot::Reference(Some(r)))
            .collect();
        heap.major_collect(&keep_roots);

        assert_eq!(heap.old_slot_count(), 100);
        assert!(
            heap.fragmentation_ratio() > Heap::OLD_COMPACT_FRAGMENTATION_THRESHOLD,
            "free list should be fragmented past threshold, was {}",
            heap.fragmentation_ratio()
        );
        assert!(heap.should_compact_old());

        let before = heap.old_slot_count();
        heap.compact_old(&keep_roots);
        let after = heap.old_slot_count();
        // Observation for the PR writeup: old-gen backing store before vs after.
        eprintln!("old-gen compaction: {before} slots -> {after} slots (25 live)");

        assert_eq!(after, 25, "store shrinks to the live count");
        assert_eq!(heap.old_live_count(), 25);
        assert!(
            (heap.fragmentation_ratio() - 0.0).abs() < f64::EPSILON,
            "fragmentation must drop to zero after compaction"
        );

        // Every survivor's data is intact and its held root remaps correctly.
        for (k, slot) in keep_roots.iter().enumerate() {
            let old_ref = slot.as_reference().unwrap();
            let new_ref = remap(&heap, old_ref);
            assert_ne!(new_ref & OLD_BIT, 0, "survivor stays in old gen");
            assert_eq!(
                heap.get(new_ref).unwrap().fields[0],
                Slot::Int(i32::try_from(k * 4).unwrap()),
                "survivor #{k} lost its field data"
            );
        }

        // The reclaimed space is one dense trailing region: 10 fresh promotions
        // land contiguously at [25..35) with no scattered holes reused.
        for i in 0..10u64 {
            let obj = Heap::make_obj("New".to_string(), vec![Slot::Int(1000)], None);
            let r = heap.promote_to_old(obj);
            assert_eq!(r & !OLD_BIT, 25 + i, "promotion must bump the dense tail");
        }
        assert_eq!(heap.old_slot_count(), 35);
    }

    /// Compaction publishes OLD→OLD forwards through `forward_map` /
    /// `has_pending_forwards`, exactly the channel the interpreter already drains
    /// to patch external roots — so simulated roots remap and read identical data.
    #[test]
    fn old_gen_compaction_exposes_forwards_for_root_patching() {
        let mut heap = Heap::new();
        let refs: Vec<u64> = (0..90)
            .map(|i| push_old(&mut heap, "Node", vec![Slot::Int(i * 7)]))
            .collect();
        // Keep every 3rd (30 survivors).
        let keep_roots: Vec<Slot> = refs
            .iter()
            .step_by(3)
            .map(|&r| Slot::Reference(Some(r)))
            .collect();
        heap.major_collect(&keep_roots);

        assert!(
            !heap.has_pending_forwards(),
            "plain mark-sweep forwards nothing"
        );

        heap.compact_old(&keep_roots);

        assert!(
            heap.has_pending_forwards(),
            "compaction must publish forwards for the interpreter to apply"
        );
        // Every published forward is an OLD→OLD mapping (no young keys leaked in).
        for (k, v) in &heap.forward_map {
            assert_ne!(k & OLD_BIT, 0, "forward key must be an old ref");
            assert_ne!(v & OLD_BIT, 0, "forward value must be an old ref");
        }

        // A held root to a moved object (refs[3] = 2nd survivor) remaps and reads
        // back the same data it carried before the move.
        let moved = refs[3];
        let remapped = remap(&heap, moved);
        assert_ne!(remapped, moved, "2nd survivor must have moved");
        assert_eq!(heap.get(remapped).unwrap().fields[0], Slot::Int(3 * 7));
    }

    /// A young object's field that points at an old object is rewritten when that
    /// old object is relocated by compaction (young→old edge).
    #[test]
    fn old_gen_compaction_rewrites_young_to_old_edges() {
        let mut heap = Heap::new();
        // Two old survivors + one garbage object between them so the second
        // survivor actually moves.
        let keep0 = push_old(&mut heap, "Old0", vec![Slot::Int(1)]);
        let garbage = push_old(&mut heap, "Garbage", vec![]);
        let keep1 = push_old(&mut heap, "Old1", vec![Slot::Int(2)]);
        let _ = garbage;

        // A young object referencing the second (moving) old survivor.
        let young = heap.allocate("Young".to_string(), 1);
        heap.write_field(young, 0, Slot::Reference(Some(keep1)))
            .unwrap();

        let keep_roots = [Slot::Reference(Some(keep0)), Slot::Reference(Some(keep1))];
        heap.compact_old(&keep_roots);

        let new_keep1 = remap(&heap, keep1);
        assert_ne!(new_keep1, keep1, "referenced old object must have moved");
        // The young object's field now points at the relocated old object.
        let field = heap.get(young).unwrap().fields[0].as_reference().unwrap();
        assert_eq!(field, new_keep1, "young→old field must be rewritten");
        assert_eq!(heap.get(field).unwrap().fields[0], Slot::Int(2));
    }

    /// An object's identity hash is preserved verbatim across an old-gen move.
    #[test]
    fn old_gen_compaction_preserves_identity_hash() {
        let mut heap = Heap::new();
        let garbage = push_old(&mut heap, "Garbage", vec![]);
        let obj = push_old(&mut heap, "Keep", vec![Slot::Int(42)]);
        let _ = garbage;

        let hash_before = heap.identity_hash(obj).unwrap();

        heap.compact_old(&[Slot::Reference(Some(obj))]);

        let moved = remap(&heap, obj);
        assert_ne!(moved, obj, "object must have moved");
        assert_eq!(
            heap.identity_hash(moved).unwrap(),
            hash_before,
            "identity hash must ride along with the moved object"
        );
    }

    /// The remembered set is re-indexed across compaction, so an old→young edge
    /// held by a relocated old object still keeps its young referent alive on the
    /// next minor GC.
    #[test]
    fn old_gen_compaction_remaps_remembered_set() {
        let mut heap = test_heap_with_capacity(64);
        let garbage = push_old(&mut heap, "Garbage", vec![]);
        let holder = push_old(&mut heap, "Holder", vec![Slot::Int(0)]);
        let _ = garbage;

        // A young object referenced only by the old `holder` (old→young edge).
        let young = heap.allocate("Payload".to_string(), 0);
        heap.write_field(holder, 0, Slot::Reference(Some(young)))
            .unwrap();
        let holder_idx = (holder & !OLD_BIT) as usize;
        assert!(heap.remembered_set.contains(&holder_idx));

        heap.compact_old(&[Slot::Reference(Some(holder))]);

        let new_holder = remap(&heap, holder);
        let new_idx = (new_holder & !OLD_BIT) as usize;
        assert_ne!(new_idx, holder_idx, "holder must have moved");
        assert!(
            heap.remembered_set.contains(&new_idx),
            "remembered set must track the holder's new index"
        );
        assert!(
            !heap.remembered_set.contains(&holder_idx),
            "stale remembered-set index must be dropped"
        );

        // The old→young edge still protects the young object on a minor GC even
        // though `young` is not directly rooted.
        run_minor_gc(&mut heap, &[Slot::Reference(Some(new_holder))]);
        let edge = heap.get(new_holder).unwrap().fields[0]
            .as_reference()
            .unwrap();
        assert_eq!(heap.get(edge).unwrap().class_name, "Payload");
    }

    /// Executor task-queue refs (bare OLD u64s) are treated as roots and
    /// rewritten across compaction, so queued-but-unrun tasks stay valid.
    #[test]
    fn old_gen_compaction_rewrites_executor_queue_refs() {
        let mut heap = Heap::new();
        // Interleave garbage so future/task both slide down.
        let _g0 = push_old(&mut heap, "Garbage", vec![]);
        let future = push_old(&mut heap, "Future", vec![]);
        let _g1 = push_old(&mut heap, "Garbage", vec![]);
        let task = push_old(&mut heap, "Runnable", vec![]);

        // The future/task survive solely via the executor's queue.
        let exec = make_executor_with_task(&mut heap, future, task);

        heap.compact_old(&[]);

        let queued = executor_task(&heap, exec);
        assert_ne!(queued.future_ref, future, "future_ref must be forwarded");
        assert_ne!(queued.task_ref, task, "task_ref must be forwarded");
        assert_eq!(heap.get(queued.future_ref).unwrap().class_name, "Future");
        assert_eq!(heap.get(queued.task_ref).unwrap().class_name, "Runnable");
    }

    /// End-to-end: minor GC promotes survivors to old, then a compacting major GC
    /// drops one and slides the rest, with every held root remapping to intact
    /// data.
    #[test]
    fn minor_promotion_then_compacting_major_preserves_integrity() {
        let mut heap = test_heap_with_capacity(8);
        heap.promotion_age = 0; // promote on first survival

        let a = heap.allocate("A".to_string(), 1);
        let b = heap.allocate("B".to_string(), 1);
        let c = heap.allocate("C".to_string(), 1);
        heap.write_field(a, 0, Slot::Int(10)).unwrap();
        heap.write_field(b, 0, Slot::Int(20)).unwrap();
        heap.write_field(c, 0, Slot::Int(30)).unwrap();

        // Minor GC: all three promote to old gen.
        let promoted = run_minor_gc(
            &mut heap,
            &[
                Slot::Reference(Some(a)),
                Slot::Reference(Some(b)),
                Slot::Reference(Some(c)),
            ],
        );
        let (pa, pc) = (promoted[0], promoted[2]);
        assert_ne!(pa.as_reference().unwrap() & OLD_BIT, 0);

        // Compacting major GC keeping only A and C — B is reclaimed, C slides.
        heap.major_collect_compacting(&[pa, pc]);

        let na = remap(&heap, pa.as_reference().unwrap());
        let nc = remap(&heap, pc.as_reference().unwrap());
        assert_eq!(heap.get(na).unwrap().fields[0], Slot::Int(10));
        assert_eq!(heap.get(nc).unwrap().fields[0], Slot::Int(30));
        assert_eq!(heap.old_slot_count(), 2, "store holds exactly A and C");
        assert_eq!(heap.old_live_count(), 2);
    }

    /// The normal `collect()` path upgrades to compaction once the old gen is
    /// fragmented past threshold, driven only through the public API the
    /// interpreter uses.
    #[test]
    fn collect_auto_compacts_when_old_gen_fragmented() {
        let mut heap = Heap::new();
        let refs: Vec<u64> = (0..100)
            .map(|i| push_old(&mut heap, "Old", vec![Slot::Int(i)]))
            .collect();
        // Root only 20 objects; collect() sweeps 80 → 0.8 fragmentation → compacts.
        let roots: Vec<Slot> = refs
            .iter()
            .step_by(5)
            .map(|&r| Slot::Reference(Some(r)))
            .collect();

        heap.collect(&roots);

        assert_eq!(
            heap.old_slot_count(),
            20,
            "collect() must compact a heavily fragmented old gen"
        );
        assert!((heap.fragmentation_ratio() - 0.0).abs() < f64::EPSILON);
        for (k, slot) in roots.iter().enumerate() {
            let new_ref = remap(&heap, slot.as_reference().unwrap());
            assert_eq!(
                heap.get(new_ref).unwrap().fields[0],
                Slot::Int(i32::try_from(k * 5).unwrap())
            );
        }
    }

    /// The minor-only fast path is unaffected: a `collect()` that stays below the
    /// fragmentation threshold never publishes an OLD forward key.
    #[test]
    fn collect_below_threshold_leaves_old_refs_untouched() {
        let mut heap = test_heap_with_capacity(8);
        heap.promotion_age = 0;
        let r = heap.allocate("Keep".to_string(), 0);
        heap.collect(&[Slot::Reference(Some(r))]);

        assert!(!heap.should_compact_old(), "tiny heap must not compact");
        for k in heap.forward_map.keys() {
            assert_eq!(k & OLD_BIT, 0, "no OLD forward keys without compaction");
        }
    }
}
#[cfg(test)]
mod fuzz;
