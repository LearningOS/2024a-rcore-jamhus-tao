//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM, mm::translated_byte_buffer, syscall::STATISITC_SYSCALL_TIMES, task::*, timer::get_time_ms
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

fn copy_in_va<T>(data: T, addr: *mut T) -> isize {
    let size = core::mem::size_of::<T>();
    let data = &data as *const _ as *const u8;
    let v = translated_byte_buffer(current_user_token(), addr as *const u8, size);
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = crate::timer::get_time_us();
    copy_in_va(TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    }, _ts);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let current = get_current_app_id();
    let stcb = get_current_tcb();
    copy_in_va(TaskInfo {
        status: stcb.task_status,
        syscall_times: STATISITC_SYSCALL_TIMES.exclusive_access()[current],
        time: if stcb.start_time == usize::MAX { 0 } else { get_time_ms() - stcb.start_time },
    }, _ti);
    0
}

use crate::mm::MapPermission;
// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    // _port: 0xwr
    // perm: 0uxwr0
    trace!("kernel: sys_mmap");
    if _port & !0x7 != 0 {
        -1
    } else if _port & 0x7 == 0 {
        -1
    } else if _start & crate::config::PAGE_SIZE - 1 != 0 {
        -1
    } else {
        let perm = MapPermission::from_bits((_port << 1) as u8).unwrap() | MapPermission::U;
        current_app_mmap(_start.into(), (_start + _len).into(), perm)
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _start & crate::config::PAGE_SIZE - 1 != 0 {
        -1
    } else {
        current_app_munmap(crate::mm::VirtAddr(_start).floor(), crate::mm::VirtAddr(_start + _len).ceil())
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
