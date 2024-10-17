//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_SBRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_TASK_INFO: usize = 410;

use alloc::vec::Vec;
use lazy_static::*;
mod fs;
use fs::*;
mod process;
use process::*;

use crate::config::MAX_SYSCALL_NUM;
lazy_static! {
    /// to statistic syscall times for each task
    pub static ref STATISITC_SYSCALL_TIMES: UPSafeCell<Vec<[u32; MAX_SYSCALL_NUM]>> = unsafe { UPSafeCell::new(Vec::new()) };
}

use crate::sync::UPSafeCell;
use crate::task::get_current_app_id;
fn record_syscall(syscall_id: usize) {
    let current = get_current_app_id();
    while STATISITC_SYSCALL_TIMES.exclusive_access().len() <= current {
        STATISITC_SYSCALL_TIMES.exclusive_access().push([0; MAX_SYSCALL_NUM]);
    }
    STATISITC_SYSCALL_TIMES.exclusive_access()[current][syscall_id] += 1;
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    record_syscall(syscall_id);
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
