//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if self.ready_queue.len() == 0 {
            return None;
        }
        // Find the smallest stride
        let mut index = 0;
        let mut rettcb: Arc<TaskControlBlock> = self.ready_queue[index].clone();
        let mut min_stride = rettcb.inner_exclusive_access().stride;
        for i in 1..self.ready_queue.len()  {
            let tcb = &self.ready_queue[i];
            let stride = tcb.inner_exclusive_access().stride;
            
            if stride == min_stride && tcb.inner_exclusive_access().prio > rettcb.inner_exclusive_access().prio {
                rettcb = tcb.clone(); 
                index = i;
            }
            else if stride < min_stride {
                min_stride = stride;
                rettcb = tcb.clone();
                index = i;
            }
        }
        // Update the stride of the next task
        self.ready_queue.remove(index);
        let pass = rettcb.inner_exclusive_access().pass;
        rettcb.inner_exclusive_access().stride = min_stride + pass;

        Some(rettcb)
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
