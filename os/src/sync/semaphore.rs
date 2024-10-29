//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_enable_deadlock_detect, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};

/// context to detect deadlock
#[derive(Clone)]
pub struct SemaphoreContext {
    sem_id: usize,
}
impl SemaphoreContext {
    /// new
    pub fn new(sem_id: usize) -> Self {
        Self {
            sem_id,
        }
    }
}


/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

/// inner
pub struct SemaphoreInner {
    /// count
    pub count: isize,
    /// wait_queue
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self, ctx: SemaphoreContext) {
        let enable_deadlock_detect = current_enable_deadlock_detect();
        trace!("kernel: Semaphore::up {{ ctx.sem_id: {}, enable_deadlock_detect: {} }}", ctx.sem_id, enable_deadlock_detect);
        if enable_deadlock_detect {
            current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().release_semaphore_resource(ctx.sem_id);
        }
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                if enable_deadlock_detect {
                    task.inner_exclusive_access().res.as_mut().unwrap().occupy_semaphore_resource(ctx.sem_id);
                }
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self, ctx: SemaphoreContext) -> isize {
        let enable_deadlock_detect = current_enable_deadlock_detect();
        trace!("kernel: Semaphore::down {{ ctx.sem_id: {}, enable_deadlock_detect: {} }}", ctx.sem_id, enable_deadlock_detect);
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        debug!("semaphore inner.count {}", inner.count);
        if inner.count < 0 {
            if enable_deadlock_detect {
                debug!("semaphore enable_deadlock_detect");
                current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().wait_semaphore_resource(ctx.sem_id);
                if !crate::task::TaskUserRes::detect_deadlock() {
                    current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().clear_waiting_resource();
                    debug!("semaphore deadlock detected");
                    return -0xdead;
                }
                debug!("semaphore finish deadlock_detect");
            }
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            debug!("semaphore block current and run next");
            block_current_and_run_next();
        } else if enable_deadlock_detect {
            current_task().unwrap().inner_exclusive_access().res.as_mut().unwrap().occupy_semaphore_resource(ctx.sem_id);
        }
        0
    }
}
