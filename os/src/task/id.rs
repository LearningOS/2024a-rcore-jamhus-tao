//! Allocator for pid, task user resource, kernel stack using a simple recycle strategy.

use super::ProcessControlBlock;
use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT_BASE, USER_STACK_SIZE};
use crate::mm::{MapPermission, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use lazy_static::*;

/// Allocator with a simple recycle strategy
pub struct RecycleAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl RecycleAllocator {
    /// Create a new allocator
    pub fn new() -> Self {
        RecycleAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }
    /// allocate a new item
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            self.current += 1;
            self.current - 1
        }
    }
    /// deallocate an item
    pub fn dealloc(&mut self, id: usize) {
        assert!(id < self.current);
        assert!(
            !self.recycled.iter().any(|i| *i == id),
            "id {} has been deallocated!",
            id
        );
        self.recycled.push(id);
    }
}

lazy_static! {
    /// Glocal allocator for pid
    static ref PID_ALLOCATOR: UPSafeCell<RecycleAllocator> =
        unsafe { UPSafeCell::new(RecycleAllocator::new()) };
    /// Global allocator for kernel stack
    static ref KSTACK_ALLOCATOR: UPSafeCell<RecycleAllocator> =
        unsafe { UPSafeCell::new(RecycleAllocator::new()) };
}

/// The idle task's pid is 0
pub const IDLE_PID: usize = 0;

/// A handle to a pid
pub struct PidHandle(pub usize);

/// Allocate a pid for a process
pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.exclusive_access().alloc())
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        // trace!("drop pid {}", self.0);
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(kstack_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - kstack_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

/// Kernel stack for a task
pub struct KernelStack(pub usize);

/// Allocate a kernel stack for a task
pub fn kstack_alloc() -> KernelStack {
    let kstack_id = KSTACK_ALLOCATOR.exclusive_access().alloc();
    let (kstack_bottom, kstack_top) = kernel_stack_position(kstack_id);
    KERNEL_SPACE.exclusive_access().insert_framed_area(
        kstack_bottom.into(),
        kstack_top.into(),
        MapPermission::R | MapPermission::W,
    );
    KernelStack(kstack_id)
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.0);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
        KSTACK_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

impl KernelStack {
    /// Push a variable of type T into the top of the KernelStack and return its raw pointer
    #[allow(unused)]
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }
    /// return the top of the kernel stack
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.0);
        kernel_stack_top
    }
}

enum WaitingResource {
    Mutex { mutex_id: usize },
    Semaphore { sem_id: usize },
    None,
}

/// User Resource for a task
pub struct TaskUserRes {
    /// task id
    pub tid: usize,
    /// user stack base
    pub ustack_base: usize,
    /// process belongs to
    pub process: Weak<ProcessControlBlock>,
    /// to detect deadlock
    lock_resource: (Vec<bool>/*for mutex*/, Vec<usize>/*for sem*/),
    waiting_resource: WaitingResource,
}
/// Return the bottom addr (low addr) of the trap context for a task
fn trap_cx_bottom_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}
/// Return the bottom addr (high addr) of the user stack for a task
fn ustack_bottom_from_tid(ustack_base: usize, tid: usize) -> usize {
    ustack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}

impl TaskUserRes {
    /// Create a new TaskUserRes (Task User Resource)
    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let tid = process.inner_exclusive_access().alloc_tid();
        let task_user_res = Self {
            tid,
            ustack_base,
            process: Arc::downgrade(&process),
            lock_resource: (Vec::new(), Vec::new()),
            waiting_resource: WaitingResource::None, 
        };
        if alloc_user_res {
            task_user_res.alloc_user_res();
        }
        task_user_res
    }
    /// Allocate user resource for a task
    pub fn alloc_user_res(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        // alloc user stack
        let ustack_bottom = ustack_bottom_from_tid(self.ustack_base, self.tid);
        let ustack_top = ustack_bottom + USER_STACK_SIZE;
        process_inner.memory_set.insert_framed_area(
            ustack_bottom.into(),
            ustack_top.into(),
            MapPermission::R | MapPermission::W | MapPermission::U,
        );
        // alloc trap_cx
        let trap_cx_bottom = trap_cx_bottom_from_tid(self.tid);
        let trap_cx_top = trap_cx_bottom + PAGE_SIZE;
        process_inner.memory_set.insert_framed_area(
            trap_cx_bottom.into(),
            trap_cx_top.into(),
            MapPermission::R | MapPermission::W,
        );
    }
    /// Deallocate user resource for a task
    fn dealloc_user_res(&self) {
        // dealloc tid
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        // dealloc ustack manually
        let ustack_bottom_va: VirtAddr = ustack_bottom_from_tid(self.ustack_base, self.tid).into();
        process_inner
            .memory_set
            .remove_area_with_start_vpn(ustack_bottom_va.into());
        // dealloc trap_cx manually
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        process_inner
            .memory_set
            .remove_area_with_start_vpn(trap_cx_bottom_va.into());
    }

    #[allow(unused)]
    /// alloc task id
    pub fn alloc_tid(&mut self) {
        self.tid = self
            .process
            .upgrade()
            .unwrap()
            .inner_exclusive_access()
            .alloc_tid();
    }
    /// dealloc task id
    pub fn dealloc_tid(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.dealloc_tid(self.tid);
    }

    /// detect deadlock
    pub fn detect_deadlock() -> bool {
        trace!("kernal:  TaskUserRes::detect_deadlock");
        let mut available = false;
        let process = crate::task::current_process();
        let process = process.inner_exclusive_access();
        let mut pass_main_thread = false;
        for task in &process.tasks {
            if let Some(task) = task.clone() {
                if !pass_main_thread {
                    pass_main_thread = true;
                    continue;
                }
                let task = task.inner_exclusive_access();
                if let Some(res) = task.res.as_ref() {
                    match res.waiting_resource {
                        WaitingResource::Mutex { mutex_id } => {
                            assert!(process.mutex_list.len() > mutex_id, "mutex resource NOT FOUND in process container");
                            if let Some((_, res)) = process.mutex_list[mutex_id].as_ref() {
                                available = *res
                            } else {
                                panic!("mutex resource NOT FOUND in process container");
                            }
                        },
                        WaitingResource::Semaphore { sem_id } => {
                            assert!(process.semaphore_list.len() > sem_id, "semaphore resource NOT FOUND in process container");
                            if let Some((_, res)) = process.semaphore_list[sem_id].as_ref() {
                                available = *res > 0
                            } else {
                                panic!("semaphore resource NOT FOUND in process container");
                            }
                        },
                        _ => available = true,
                    }
                }
                if available {
                    break;
                }
            }
        }
        available
    }
    /// wait mutex resource
    pub fn wait_mutex_resource(&mut self, mutex_id: usize) {
        trace!("kernal:  TaskUserRes::wait_mutex_resource");
        match self.waiting_resource {
            WaitingResource::None => self.waiting_resource = WaitingResource::Mutex { mutex_id },
            _ => panic!("Please clear waiting resource first"),
        }
    }
    /// wait semaphore resource
    pub fn wait_semaphore_resource(&mut self, sem_id: usize) {
        trace!("kernal:  TaskUserRes::wait_semaphore_resource");
        match self.waiting_resource {
            WaitingResource::None => self.waiting_resource = WaitingResource::Semaphore { sem_id },
            _ => panic!("Please clear waiting resource first"),
        }
    }
    /// clear waiting resource
    pub fn clear_waiting_resource(&mut self) {
        trace!("kernal:  TaskUserRes::clear_waiting_resource");
        self.waiting_resource = WaitingResource::None;
    }

    /// occupy mutex resource from process container
    pub fn occupy_mutex_resource(&mut self, mutex_id: usize) -> bool {
        trace!("kernal:  TaskUserRes::occupy_mutex_resource {{ mutex_id: {} }}", mutex_id);
        self.clear_waiting_resource();

        let process = self.process.upgrade().unwrap();
        let mut inner = process.inner_exclusive_access();
        assert!(inner.mutex_list.len() > mutex_id, "mutex resource NOT FOUND in process container");
        if let Some((_, res)) = inner.mutex_list[mutex_id].as_mut() {
            if *res {
                *res = false;
                while self.lock_resource.0.len() <= mutex_id {
                    self.lock_resource.0.push(false);
                }
                self.lock_resource.0[mutex_id] = true;
                true
            } else {
                false
            }
        } else {
            panic!("mutex resource NOT FOUND in process container");
        }
    }
    /// release mutex resource to process container
    pub fn release_mutex_resource(&mut self, mutex_id: usize) {
        trace!("kernal:  TaskUserRes::release_mutex_resource {{ mutex_id: {} }}", mutex_id);
        self.clear_waiting_resource();

        let process = self.process.upgrade().unwrap();
        let mut inner = process.inner_exclusive_access();
        assert!(inner.mutex_list.len() > mutex_id, "mutex resource NOT FOUND in process container");
        assert!(self.lock_resource.0.len() > mutex_id, "release mutex resource before occupied");
        // while self.lock_resource.0.len() <= mutex_id {
        //     self.lock_resource.0.push(false);
        // }
        if let Some((_, res)) = inner.mutex_list[mutex_id].as_mut() {
            if self.lock_resource.0[mutex_id] {
                *res = true;
                self.lock_resource.0[mutex_id] = false;
            }
        } else {
            panic!("mutex resource NOT FOUND in process container");
        }
    }

    /// occupy semaphore resource in process container
    pub fn occupy_semaphore_resource(&mut self, sem_id: usize) -> bool {
        trace!("kernal:  TaskUserRes::occupy_semaphore_resource {{ sem_id: {} }}", sem_id);
        self.clear_waiting_resource();

        let process = self.process.upgrade().unwrap();
        let mut inner = process.inner_exclusive_access();
        assert!(inner.semaphore_list.len() > sem_id, "semaphore resource NOT FOUND in process container");
        if let Some((_, res)) = inner.semaphore_list[sem_id].as_mut() {
            if *res > 0 {
                *res -= 1;
                while self.lock_resource.1.len() <= sem_id {
                    self.lock_resource.1.push(0);
                }
                self.lock_resource.1[sem_id] += 1;
                true
            } else {
                false
            }
        } else {
            panic!("semaphore resource NOT FOUND in process container");
        }
    }
    /// release semaphore resource in process container
    pub fn release_semaphore_resource(&mut self, sem_id: usize) {
        trace!("kernal:  TaskUserRes::release_semaphore_resource {{ sem_id: {} }}", sem_id);
        self.clear_waiting_resource();

        let process = self.process.upgrade().unwrap();
        let mut inner = process.inner_exclusive_access();
        assert!(inner.semaphore_list.len() > sem_id, "semaphore resource NOT FOUND in process container");
        assert!(self.lock_resource.1.len() > sem_id, "release semaphore before occupied");
        // while self.lock_resource.1.len() <= sem_id {
        //     self.lock_resource.1.push(0);
        // }
        if let Some((_, res)) = inner.semaphore_list[sem_id].as_mut() {
            if self.lock_resource.1[sem_id] > 0 {
                *res += 1;
                self.lock_resource.1[sem_id] -= 1;
            }
        } else {
            panic!("semaphore resource NOT FOUND in process container");
        }
    }

    // /// block all lock resource, invoke when goto BLOCK status
    // pub fn block_all_lock_resource(&mut self) {
    //     trace!("kernal:  TaskUserRes::block_all_lock_resource");
    //     todo!();
    // }
    /// drop all lock resource, invoke when goto non-BLOCK status
    pub fn drop_all_lock_resource(&mut self) {
        trace!("kernal:  TaskUserRes::drop_all_lock_resource");
        self.clear_waiting_resource();

        let process = self.process.upgrade().unwrap();
        let mut inner = process.inner_exclusive_access();
        assert!(self.lock_resource.0.len() <= inner.mutex_list.len(), "It's weird!");
        assert!(self.lock_resource.1.len() <= inner.semaphore_list.len(), "It's weird!");
    
        let mut it = self.lock_resource.0.iter();
        let mut container_it = inner.mutex_list.iter_mut();
        while let Some(res) = it.next() {
            if let Some(Some((_, container_res))) = container_it.next() {
                // the outer `Some` must be met
                // if the inner `Some` does't met, continue both `it` && `container_it`
                *container_res ^= *res;  // boolean addition
            }
        }
        self.lock_resource.0.clear();
    
        let mut it = self.lock_resource.1.iter();
        let mut container_it = inner.semaphore_list.iter_mut();
        while let Some(res) = it.next() {
            if let Some(Some((_, container_res))) = container_it.next() {
                *container_res += *res;
            }
        }
        self.lock_resource.1.clear();
    }

    /// The bottom usr vaddr (low addr) of the trap context for a task with tid
    pub fn trap_cx_user_va(&self) -> usize {
        trap_cx_bottom_from_tid(self.tid)
    }
    /// The physical page number(ppn) of the trap context for a task with tid
    pub fn trap_cx_ppn(&self) -> PhysPageNum {
        let process = self.process.upgrade().unwrap();
        let process_inner = process.inner_exclusive_access();
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        process_inner
            .memory_set
            .translate(trap_cx_bottom_va.into())
            .unwrap()
            .ppn()
    }
    /// the bottom addr (low addr) of the user stack for a task
    pub fn ustack_base(&self) -> usize {
        self.ustack_base
    }
    /// the top addr (high addr) of the user stack for a task
    pub fn ustack_top(&self) -> usize {
        ustack_bottom_from_tid(self.ustack_base, self.tid) + USER_STACK_SIZE
    }
}

impl Drop for TaskUserRes {
    fn drop(&mut self) {
        self.dealloc_tid();
        self.dealloc_user_res();
        self.drop_all_lock_resource();
    }
}
