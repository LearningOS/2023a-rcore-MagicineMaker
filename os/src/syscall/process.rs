//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    fs::{open_file, OpenFlags},
    mm::{
        translated_refmut, translated_str, PageTable, current_insert_area, current_remove_area,
        VirtAddr, MapPermission},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, TaskControlBlock,
    },
    timer::get_time_us,
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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let ts = translated_refmut(current_user_token(), _ts);
    let us = crate::timer::get_time_us();
    *ts = TimeVal{
        sec: us / 1000_000,
        usec: us % 1000_000, 
    };
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _len == 0 {
        return 0;
    }

    let va_start = VirtAddr(_start);
    let va_end = VirtAddr(_start + _len);

    if _port & !0x7 != 0 ||  _port & 0x7 == 0 || !va_start.aligned() {
        return -1;
    }

    let mut flags = MapPermission::U;
    if _port & 1 != 0 {flags |= MapPermission::R;}
    if _port & 2 != 0 {flags |= MapPermission::W;}
    if _port & 4 != 0 {flags |= MapPermission::X;}

    let pt = PageTable::from_token(current_user_token());
    let mut va = va_start;
    while va < va_end {
        let vpn = va.floor();
        let pte = pt.translate(vpn);
        if pte.is_some() && pte.unwrap().is_valid() {
            return -1;
        }
        va.0 += PAGE_SIZE;
    } 

    current_insert_area(va_start, va_end, flags);

    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let va_start = VirtAddr(_start);
    let va_end = VirtAddr(_start + _len);

    if !va_start.aligned() {
        return -1;
    }

    let mut va = va_start;
    let pt = PageTable::from_token(current_user_token());
    while va < va_end {
        let vpn = va.floor();
        let pte = pt.translate(vpn);
        if pte.is_none() || !pte.unwrap().is_valid() {
            return -1;
        }
        va.0 += PAGE_SIZE;
    }

    current_remove_area(va_start, va_end);

    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    let path = translated_str(current_user_token(), _path);
    let app_inode = open_file(path.as_str(), OpenFlags::RDONLY);
        
    if app_inode.is_none() {return -1;}
    let app_inode_arc = app_inode.unwrap();
    let data = (*app_inode_arc).read_all();

    let task = TaskControlBlock::new(data.as_slice());

    let current_task = current_task().unwrap();
    task.inner_exclusive_access().parent = Some(Arc::downgrade(&current_task));
    let task_arc = Arc::new(task);
    current_task.inner_exclusive_access().children.push(task_arc.clone());

    add_task(task_arc.clone());
    task_arc.as_ref().getpid() as isize
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio < 2 {
        return -1;
    }
    let prio = _prio as usize;
    current_task().unwrap().inner_exclusive_access().prio = prio;
    current_task().unwrap().inner_exclusive_access().pass = crate::config::BIG_STRIDE / prio;
    _prio
}
