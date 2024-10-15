//! Process management syscalls

pub mod sys_exit {
    use crate::task::exit_current_and_run_next;

    /// task exits and submit an exit code
    pub fn process(exit_code: i32) -> ! {
        trace!("[kernel] Application exited with code {}", exit_code);
        exit_current_and_run_next();
        panic!("Unreachable in sys_exit!");
    }
}

pub mod sys_yield {
    use crate::task::suspend_current_and_run_next;

    /// current task gives up resources for other tasks
    pub fn process() -> isize {
        trace!("kernel: sys_yield");
        suspend_current_and_run_next();
        0
    }
}

pub mod sys_get_time {
    use crate::timer::get_time_us;

    #[repr(C)]
    #[derive(Debug)]
    pub struct TimeVal {
        pub sec: usize,
        pub usec: usize,
    }

    /// get time with second and microsecond
    pub fn process(ts: *mut TimeVal, _tz: usize) -> isize {
        trace!("kernel: sys_get_time");
        let us = get_time_us();
        unsafe {
            *ts = TimeVal {
                sec: us / 1_000_000,
                usec: us % 1_000_000,
            };
        }
        0
    }
}

pub mod sys_task_info {
    use crate::config::MAX_SYSCALL_NUM;
    use crate::syscall::TCB_TO_SYSCALL;
    use crate::task::TaskStatus;
    use crate::task::get_current_tcb;
    use crate::timer::get_time_ms;

    /// Task information
    pub struct TaskInfo {
        /// Task status in it's life cycle
        status: TaskStatus,
        /// The numbers of syscall called by task
        syscall_times: [u32; MAX_SYSCALL_NUM],
        /// Total running time of task
        time: usize,
    }
    
    /// YOUR JOB: Finish sys_task_info to pass testcases
    pub fn process(_ti: *mut TaskInfo) -> isize {
        trace!("kernel: sys_task_info");
        let tcb = get_current_tcb();
        unsafe {
            (*_ti).status = tcb.task_status;
            for (i, x) in tcb.syscall_times.iter().enumerate() {
                (*_ti).syscall_times[TCB_TO_SYSCALL[i]] = *x;
            }
            // (*_ti).syscall_times = tcb.syscall_times;
            (*_ti).time = if tcb.time == usize::MAX { 0 } else { get_time_ms() - tcb.time };
        }
        0
    }
}
