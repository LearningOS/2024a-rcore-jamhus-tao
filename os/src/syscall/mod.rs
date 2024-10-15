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
const SYSCALL_TASK_INFO: usize = 410;

use crate::config::MAX_SYSCALL_NUM;
/// mapping from SYSCALL to TCB_SYSCALL
pub const SYSCALL_TO_TCB: [usize; MAX_SYSCALL_NUM] = {
    let mut ret = [0; MAX_SYSCALL_NUM];
    ret[SYSCALL_WRITE] = 0;
    ret[SYSCALL_EXIT] = 1;
    ret[SYSCALL_YIELD] = 2;
    ret[SYSCALL_GET_TIME] = 3;
    ret[SYSCALL_TASK_INFO] = 4;
    ret
};

use crate::config::MAX_TCB_SYSCALL_NUM;
/// mapping from TCB_SYSCALL to SYSCALL
pub const TCB_TO_SYSCALL: [usize; MAX_TCB_SYSCALL_NUM] = {
    let mut ret = [0; MAX_TCB_SYSCALL_NUM];
    ret[0] = SYSCALL_WRITE;
    ret[1] = SYSCALL_EXIT;
    ret[2] = SYSCALL_YIELD;
    ret[3] = SYSCALL_GET_TIME;
    ret[4] = SYSCALL_TASK_INFO;
    ret
};

mod fs;
mod process;

// mod sys_write;
// mod sys_exit;
// mod sys_yield;
// mod sys_get_time;
// mod sys_task_info;
use crate::task::record_syscall;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    record_syscall(syscall_id);
    match syscall_id {
        SYSCALL_WRITE => fs::sys_write::process(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => process::sys_exit::process(args[0] as i32),
        SYSCALL_YIELD => process::sys_yield::process(),
        SYSCALL_GET_TIME => process::sys_get_time::process(args[0] as *mut process::sys_get_time::TimeVal, args[1]),
        SYSCALL_TASK_INFO => process::sys_task_info::process(args[0] as *mut process::sys_task_info::TaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
