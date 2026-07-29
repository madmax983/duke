//! `duke-runtime::sync` — Synchronization primitives and extensions

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Extension trait for `Mutex` to gracefully handle poisoned locks.
pub trait LockExt<T: ?Sized> {
    /// Acquires the lock, ignoring any poisoning.
    fn lock_poison_free(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> LockExt<T> for Mutex<T> {
    fn lock_poison_free(&self) -> MutexGuard<'_, T> {
        self.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Extension trait for `RwLock` to gracefully handle poisoned locks.
pub trait RwLockExt<T: ?Sized> {
    /// Acquires the read lock, ignoring any poisoning.
    fn read_poison_free(&self) -> RwLockReadGuard<'_, T>;
    /// Acquires the write lock, ignoring any poisoning.
    fn write_poison_free(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> RwLockExt<T> for RwLock<T> {
    fn read_poison_free(&self) -> RwLockReadGuard<'_, T> {
        self.read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write_poison_free(&self) -> RwLockWriteGuard<'_, T> {
        self.write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
