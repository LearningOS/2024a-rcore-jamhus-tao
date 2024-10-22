//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_priority_queue: Vec<VecDeque<Arc<TaskControlBlock>>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_priority_queue: Vec::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        let priority = task.inner_exclusive_access().priority;
        assert!(crate::config::MIN_STRIDE_PRIORITY <= priority && priority <= crate::config::MAX_STRIDE_PRIORITY,
                "priority should in range [MIN_STRIDE_PRIORITY, MAX_STRIDE_PRIORITY]");
        let index = crate::config::MAX_STRIDE_PRIORITY - priority;
        while self.ready_priority_queue.len() <= index {
            self.ready_priority_queue.push(VecDeque::new());
        }
        self.ready_priority_queue[index].push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        for it in self.ready_priority_queue.iter_mut().rev() {
            if !it.is_empty() {
                return it.pop_front();
            }
        }
        None
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
