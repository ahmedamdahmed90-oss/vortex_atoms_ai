// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//! Engine pool: N independent `LlmInference` slots for concurrent generate.
//!
//! Each slot owns a full weight set (candle `ModelWeights` is not `Clone`;
//! KV lives inside the model), so the pool trades RAM for execution
//! parallelism. Slots are handed out by [`EnginePool::checkout`]: a free-list
//! of indices with async wait when empty, returned on [`PooledEngine`] drop
//! (including panic). Admission above the pool is still the API semaphore
//! (`max(pool_size, MAX_CONCURRENT_INFERENCE)`), so a pool of 1 preserves the
//! historical "one runs, one waits on the slot" queueing while `pool_size ≥ 2`
//! yields true parallel generation.
//!
//! Model swaps bump a config epoch and publish the new `LlmConfig`. The slot
//! that performed the swap is marked current; every other slot reloads lazily
//! on its next checkout (never mid-generation). Health / device / list read
//! any slot via try-read and never queue behind a running generate.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{Notify, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::llm_config::LlmConfig;
use crate::llm_inference::LlmInference;
use crate::Result;

/// One engine slot plus the config epoch it was last loaded/refreshed at.
struct Slot {
    engine: Arc<RwLock<LlmInference>>,
    loaded_epoch: AtomicU64,
}

/// Shared pool state behind `Arc` so [`PooledEngine`] can check in on drop.
struct PoolInner {
    slots: Vec<Slot>,
    /// Free slot indices (LIFO). Sync mutex: check-in runs in `Drop`.
    free: Mutex<Vec<usize>>,
    /// Wakes a waiter when a slot returns to the free list.
    notify: Notify,
    /// Bumped on every published model config; slots with `loaded_epoch < epoch`
    /// reload on checkout.
    epoch: AtomicU64,
    /// Latest desired model config (None until first swap; boot config is
    /// already loaded into every slot at epoch 0).
    current_config: Mutex<Option<LlmConfig>>,
}

/// Fail-fast-free checkout guard: returns the slot to the pool on drop.
pub struct PooledEngine {
    inner: Arc<PoolInner>,
    index: usize,
    engine: Arc<RwLock<LlmInference>>,
}

impl PooledEngine {
    /// Slot index within the pool (stable for this checkout).
    pub fn index(&self) -> usize {
        self.index
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, LlmInference> {
        self.engine.write().await
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, LlmInference> {
        self.engine.read().await
    }

    /// Blocking write for `spawn_blocking` / `block_in_place` callers.
    pub fn blocking_write(&self) -> RwLockWriteGuard<'_, LlmInference> {
        self.engine.blocking_write()
    }
}

impl Drop for PooledEngine {
    fn drop(&mut self) {
        self.inner
            .free
            .lock()
            .expect("engine pool free-list poisoned")
            .push(self.index);
        self.inner.notify.notify_one();
    }
}

/// Pool of independent engines (see module docs).
#[derive(Clone)]
pub struct EnginePool {
    inner: Arc<PoolInner>,
}

impl EnginePool {
    /// Load `size` independent engines from the same config (size is clamped
    /// to `1..=MAX_POOL_SIZE` by callers via `effective_pool_size`).
    pub fn load(config: &LlmConfig, size: usize) -> Result<Self> {
        let size = size.max(1);
        let mut slots = Vec::with_capacity(size);
        let mut free = Vec::with_capacity(size);
        for i in 0..size {
            if i > 0 {
                println!("[EnginePool] Loading slot {}/{size}...", i + 1);
            }
            let engine = LlmInference::load(config)?;
            slots.push(Slot {
                engine: Arc::new(RwLock::new(engine)),
                loaded_epoch: AtomicU64::new(0),
            });
            free.push(i);
        }
        // LIFO free-list: pop yields the last pushed (highest index) first —
        // irrelevant for correctness; deterministic for tests.
        free.reverse();
        Ok(Self {
            inner: Arc::new(PoolInner {
                slots,
                free: Mutex::new(free),
                notify: Notify::new(),
                epoch: AtomicU64::new(0),
                current_config: Mutex::new(None),
            }),
        })
    }

    pub fn len(&self) -> usize {
        self.inner.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.slots.is_empty()
    }

    /// Number of slots currently free (not checked out).
    pub fn free_count(&self) -> usize {
        self.inner
            .free
            .lock()
            .expect("engine pool free-list poisoned")
            .len()
    }

    /// Check out a slot, waiting async until one is free. Lazy-reloads the
    /// slot first if a model swap published a newer config epoch.
    ///
    /// Callers must hold an API admission permit so the waiter set stays
    /// bounded (see `ApiState` concurrency model).
    pub async fn checkout(&self) -> PooledEngine {
        let index = self.acquire_index().await;
        let engine = Arc::clone(&self.inner.slots[index].engine);
        self.refresh_slot(index, &engine).await;
        PooledEngine {
            inner: Arc::clone(&self.inner),
            index,
            engine,
        }
    }

    /// Non-blocking checkout for callers that must fail fast (none today;
    /// admission is the semaphore). Returns `None` when every slot is busy.
    pub fn try_checkout(&self) -> Option<PooledEngine> {
        let index = self.try_acquire_index()?;
        let engine = Arc::clone(&self.inner.slots[index].engine);
        // Sync path skips lazy reload: only used when the caller already
        // knows the config is current (or will handle errors). Prefer
        // `checkout` for generate paths.
        Some(PooledEngine {
            inner: Arc::clone(&self.inner),
            index,
            engine,
        })
    }

    /// Fork a session from any slot's config (read brief; never queues long
    /// behind generate — falls through slots on write-locked ones).
    pub async fn fork_session(&self) -> crate::inference_session::InferenceSession {
        for slot in &self.inner.slots {
            if let Ok(guard) = slot.engine.try_read() {
                return guard.fork_session();
            }
        }
        // All write-locked: wait on slot 0 (rare; only during generate).
        let guard = self.inner.slots[0].engine.read().await;
        guard.fork_session()
    }

    /// try-read metadata from any idle slot; `None` only when every slot is
    /// write-locked (all generating) — callers map that to 503 busy.
    pub fn try_read_any(&self) -> Option<RwLockReadGuard<'_, LlmInference>> {
        for slot in &self.inner.slots {
            if let Ok(guard) = slot.engine.try_read() {
                return Some(guard);
            }
        }
        None
    }

    /// Publish a new model config: bump the epoch and store it for lazy
    /// reload. `applied_index` is the slot that already swapped in-place
    /// (marked current so it does not reload itself).
    pub fn publish_config(&self, config: LlmConfig, applied_index: Option<usize>) {
        let epoch = self.inner.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        *self
            .inner
            .current_config
            .lock()
            .expect("engine pool current_config poisoned") = Some(config);
        if let Some(i) = applied_index {
            // Bounds-check: a stale/foreign index must not panic the API.
            if let Some(slot) = self.inner.slots.get(i) {
                slot.loaded_epoch.store(epoch, Ordering::Release);
            }
        }
    }

    /// Swap every slot to `config` (admin `/v1/models/swap` and pool-wide
    /// routing). Write-locks each slot sequentially — waits for in-flight
    /// generates one slot at a time, never holds two write guards at once
    /// (no deadlock with single-slot checkouts).
    pub async fn swap_all(&self, config: &LlmConfig) -> Result<()> {
        // Publish first so any concurrent checkout of a not-yet-swapped slot
        // sees the new epoch (it would reload the same config — harmless).
        self.publish_config(config.clone(), None);
        for slot in &self.inner.slots {
            let mut guard = slot.engine.write().await;
            tokio::task::block_in_place(|| guard.swap_model(config))?;
            slot.loaded_epoch
                .store(self.inner.epoch.load(Ordering::Acquire), Ordering::Release);
            drop(guard);
        }
        Ok(())
    }

    async fn acquire_index(&self) -> usize {
        loop {
            // Register interest before the pop so a concurrent check-in's
            // notify_one cannot be lost between pop and await.
            let notified = self.inner.notify.notified();
            if let Some(index) = self
                .inner
                .free
                .lock()
                .expect("engine pool free-list poisoned")
                .pop()
            {
                return index;
            }
            notified.await;
        }
    }

    fn try_acquire_index(&self) -> Option<usize> {
        self.inner
            .free
            .lock()
            .expect("engine pool free-list poisoned")
            .pop()
    }

    async fn refresh_slot(&self, index: usize, engine: &Arc<RwLock<LlmInference>>) {
        let epoch = self.inner.epoch.load(Ordering::Acquire);
        if self.inner.slots[index].loaded_epoch.load(Ordering::Acquire) >= epoch {
            return;
        }
        let config = self
            .inner
            .current_config
            .lock()
            .expect("engine pool current_config poisoned")
            .clone();
        let Some(config) = config else {
            // Epoch bumped without a config (should not happen); mark current.
            self.inner.slots[index]
                .loaded_epoch
                .store(epoch, Ordering::Release);
            return;
        };
        let mut guard = engine.write().await;
        match tokio::task::block_in_place(|| guard.swap_model(&config)) {
            Ok(()) => {
                self.inner.slots[index]
                    .loaded_epoch
                    .store(epoch, Ordering::Release);
            }
            Err(e) => {
                eprintln!(
                    "[EnginePool] Lazy reload of slot {index} failed: {}",
                    e.public_message()
                );
                // Leave epoch stale; next checkout retries. Generate on the
                // old weights is better than failing the request outright.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free_list_roundtrip() {
        let inner = PoolInner {
            slots: vec![],
            free: Mutex::new(vec![0, 1, 2]),
            notify: Notify::new(),
            epoch: AtomicU64::new(0),
            current_config: Mutex::new(None),
        };
        let arc = Arc::new(inner);
        assert_eq!(
            arc.free.lock().unwrap().len(),
            3,
            "starts with all slots free"
        );
        let popped = arc.free.lock().unwrap().pop();
        assert_eq!(popped, Some(2), "LIFO pop");
        arc.free.lock().unwrap().push(2);
        assert_eq!(
            arc.free.lock().unwrap().len(),
            3,
            "push restores free count"
        );
    }

    #[test]
    fn test_free_list_lifo_roundtrip() {
        free_list_roundtrip();
    }

    #[test]
    fn test_publish_config_bumps_epoch_and_marks_applied_slot() {
        let inner = Arc::new(PoolInner {
            slots: vec![],
            free: Mutex::new(vec![]),
            notify: Notify::new(),
            epoch: AtomicU64::new(0),
            current_config: Mutex::new(None),
        });
        let pool = EnginePool { inner };
        assert_eq!(pool.inner.epoch.load(Ordering::Acquire), 0);

        let cfg = LlmConfig::default();
        // Out-of-range applied index must not panic (bounds-checked store).
        pool.publish_config(cfg.clone(), Some(1));
        assert_eq!(pool.inner.epoch.load(Ordering::Acquire), 1);
        let stored = pool
            .inner
            .current_config
            .lock()
            .unwrap()
            .clone()
            .expect("config published");
        assert_eq!(stored.model_path, cfg.model_path);
    }

    #[test]
    fn test_try_checkout_none_when_no_slots() {
        let inner = Arc::new(PoolInner {
            slots: vec![],
            free: Mutex::new(vec![]),
            notify: Notify::new(),
            epoch: AtomicU64::new(0),
            current_config: Mutex::new(None),
        });
        let pool = EnginePool { inner };
        assert!(pool.try_checkout().is_none());
        assert_eq!(pool.free_count(), 0);
    }
}
