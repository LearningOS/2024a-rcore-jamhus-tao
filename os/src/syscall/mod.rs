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

const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_SBRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_TASK_INFO: usize = 410;

mod fs;
use fs::*;
mod process;
use process::*;

use alloc::vec::Vec;
use lazy_static::*;

use crate::config::MAX_SYSCALL_NUM;
lazy_static! {
    /// to statistic syscall times for each task
    pub static ref STATISITC_SYSCALL_TIMES: UPSafeCell<Vec<[u32; MAX_SYSCALL_NUM]>> = unsafe { UPSafeCell::new(Vec::new()) };
}

use crate::sync::UPSafeCell;
use crate::task::current_task;
fn record_syscall(syscall_id: usize) {
    let pid = current_task().unwrap().pid.0;
    while STATISITC_SYSCALL_TIMES.exclusive_access().len() <= pid {
        STATISITC_SYSCALL_TIMES.exclusive_access().push([0; MAX_SYSCALL_NUM]);
    }
    STATISITC_SYSCALL_TIMES.exclusive_access()[pid][syscall_id] += 1;
}

use crate::fs::Stat;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 4]) -> isize {
    record_syscall(syscall_id);
    match syscall_id {
        SYSCALL_OPEN => sys_open(args[1] as *const u8, args[2] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_LINKAT => sys_linkat(args[1] as *const u8, args[3] as *const u8),
        SYSCALL_UNLINKAT => sys_unlinkat(args[1] as *const u8),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut Stat),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as isize),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

///
pub fn copy_in_va<T>(data: T, addr: *mut T) -> isize {
  let size = core::mem::size_of::<T>();
  let data = &data as *const _ as *const u8;
  let v = crate::mm::translated_byte_buffer(crate::task::current_user_token(), addr as *const u8, size);
  let mut i = 0;
  for buffer in v {
      for byte in buffer {
          if i == size {
              break;
          }
          unsafe {
              *byte = *data.add(i);
              i += 1;
          }
      }
  }
  0
}