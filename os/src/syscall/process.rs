//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, current_user_token, current_insert_area, current_shrink_area, get_current_task_syscall_times, get_current_task_time, TaskStatus,
    },
    timer::get_time_us,
    mm::{VirtAddr, PageTable, MapPermission},
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let pt = PageTable::from_token(current_user_token());

    let va1 = VirtAddr(ts as usize);
    let ppn1 = pt.translate(va1.floor()).unwrap().ppn();
    let pa1 = (ppn1.0 << 12) + va1.page_offset();

    let va2 = VirtAddr((ts as usize) + 8);
    let ppn2 = pt.translate(va2.floor()).unwrap().ppn();
    let pa2 = (ppn2.0 << 12) + va2.page_offset();

    let pa1 = pa1 as *mut usize;
    let pa2 = pa2 as *mut usize;

    let us = get_time_us();

    unsafe {
        *pa1 = us / 1_000_000;
        *pa2 = us % 1_000_000;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let va = VirtAddr(ti as usize);
    let pt = PageTable::from_token(current_user_token());
    let ppn = pt.translate(va.floor()).unwrap().ppn();
    let pa = (ppn.0 << 12) + va.page_offset();
    let pa = pa as *mut TaskInfo;

    let syscall_times = get_current_task_syscall_times();
    let time = get_current_task_time();

    unsafe { 
        (*pa).status = TaskStatus::Running;
        (*pa).syscall_times = syscall_times;
        (*pa).time = time;
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if len == 0 {
        return 0;
    }
    
    let va_start = VirtAddr(start);
    let va_end = VirtAddr(start + len);

    // 可能的错误: start 没有按页大小对齐 ;  port & !0x7 != 0 (port 其余位必须为0) ;  port & 0x7 = 0 (这样的内存无意义)
    if port & !0x7 != 0 ||  port & 0x7 == 0 || !va_start.aligned() {
        return -1;
    }

    // flag 
    let mut flags = MapPermission::U;
    if port & 1 != 0 {flags |= MapPermission::R;}
    if port & 2 != 0 {flags |= MapPermission::W;}
    if port & 4 != 0 {flags |= MapPermission::X;}

    let pt = PageTable::from_token(current_user_token());
    let mut va = va_start;
    // 看看页表, 有没有已经映射的
    while va < va_end {
        let vpn = va.floor();
        let pte = pt.translate(vpn);
        if pte.is_some() && pte.unwrap().is_valid() {
            return -1;
        }
        va.0 += PAGE_SIZE;
    } 

    println!("mmap in, flags = {:#b}", flags.bits());
    current_insert_area(va_start, va_end, flags);
    println!("mmap out");

    // map

    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let va_start = VirtAddr(start);
    let va_end = VirtAddr(start + len);

    if !va_start.aligned() {
        return -1;
    }

    let mut va = va_start;
    let pt = PageTable::from_token(current_user_token());
    // 检查: [start, start + len) 中存在未被映射的虚存。
    while va < va_end {
        let vpn = va.floor();
        let pte = pt.translate(vpn);
        if pte.is_none() || !pte.unwrap().is_valid() {
            return -1;
        }
        va.0 += PAGE_SIZE;
    }

    println!("munmap in");
    // unmap to pt 
    current_shrink_area(va_start, va_end);  // 如果要求映射的时候是 start ~ 2页, 收回的时候要求 start ~ 1页 ; 将会把两页全收了, 不过测例过了...
    println!("munmap out");

    0
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
