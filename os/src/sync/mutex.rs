//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::{current_enable_deadlock_detect, TaskControlBlock};
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// context to detect deadlock
#[derive(Clone)]
pub struct MutexContext {
    mutex_id: usize,
}
impl MutexContext {
    /// new
    pub fn new(mutex_id: usize) -> Self {
        Self {
            mutex_id
        }
    }
}

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self, ctx: MutexContext) -> isize;
    /// Unlock the mutex
    fn unlock(&self, ctx: MutexContext);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self, ctx: MutexContext) -> isize {
        let enable_deadlock_detect = current_enable_deadlock_detect();
        trace!("kernel: MutexSpin::lock {{ ctx.mutex_id: {}, enable_deadlock_detect{} }}", ctx.mutex_id, enable_deadlock_detect);
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                if enable_deadlock_detect {
                    current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().wait_mutex_resource(ctx.mutex_id);
                    if !crate::task::TaskUserRes::detect_deadlock() {
                        current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().clear_waiting_resource();
                        return -0xdead;
                    }
                }
                suspend_current_and_run_next();
                continue;
            } else {
                if enable_deadlock_detect {
                    current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().occupy_mutex_resource(ctx.mutex_id);
                }
                *locked = true;
                return 0;
            }
        }
    }

    fn unlock(&self, ctx: MutexContext) {
        let enable_deadlock_detect = current_enable_deadlock_detect();
        trace!("kernel: MutexSpin::unlock {{ ctx.mutex_id: {}, enable_deadlock_detect: {} }}", ctx.mutex_id, enable_deadlock_detect);
        if enable_deadlock_detect {
            current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().release_mutex_resource(ctx.mutex_id);
        }
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

/// inner
pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self, ctx: MutexContext) -> isize {
        let enable_deadlock_detect = current_enable_deadlock_detect();
        trace!("kernel: MutexBlocking::lock {{ ctx.mutex_id: {}, enable_deadlock_detect: {} }}", ctx.mutex_id, enable_deadlock_detect);
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            if enable_deadlock_detect {
                current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().wait_mutex_resource(ctx.mutex_id);
                if !crate::task::TaskUserRes::detect_deadlock() {
                    current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().clear_waiting_resource();
                    return -0xdead;
                }
            }
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            if enable_deadlock_detect {
                current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().occupy_mutex_resource(ctx.mutex_id);
            }
            mutex_inner.locked = true;
        }
        0
    }

    /// unlock the blocking mutex
    fn unlock(&self, ctx: MutexContext) {
        let enable_deadlock_detect = current_enable_deadlock_detect();
        trace!("kernel: MutexBlocking::unlock {{ ctx.mutex_id: {}, enable_deadlock_detect: {} }}", ctx.mutex_id, enable_deadlock_detect);
        if enable_deadlock_detect {
            current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().release_mutex_resource(ctx.mutex_id);
        }
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            if enable_deadlock_detect {
                waking_task.inner_exclusive_access().res.as_mut().unwrap().occupy_mutex_resource(ctx.mutex_id);
            }
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
