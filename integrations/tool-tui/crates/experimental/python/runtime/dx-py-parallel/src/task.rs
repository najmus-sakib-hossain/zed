//! Task definitions for the parallel executor

use std::any::Any;

/// Priority levels for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A task that can be executed by the parallel executor
pub struct Task {
    /// The closure to execute
    work: Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>,
    /// Task priority
    pub priority: TaskPriority,
}

impl Task {
    /// Create a new task with normal priority
    pub fn new<F, R>(f: F) -> Self
    where
        F: FnOnce() -> R + Send + 'static,
        R: Any + Send + 'static,
    {
        Self {
            work: Box::new(move || Box::new(f()) as Box<dyn Any + Send>),
            priority: TaskPriority::Normal,
        }
    }

    /// Create a new task with specified priority
    pub fn with_priority<F, R>(f: F, priority: TaskPriority) -> Self
    where
        F: FnOnce() -> R + Send + 'static,
        R: Any + Send + 'static,
    {
        Self {
            work: Box::new(move || Box::new(f()) as Box<dyn Any + Send>),
            priority,
        }
    }

    /// Execute the task and return the result
    pub fn execute(self) -> Box<dyn Any + Send> {
        (self.work)()
    }
}

/// Result handle for a submitted task
pub struct TaskHandle<T> {
    receiver: crossbeam::channel::Receiver<T>,
}

impl<T> TaskHandle<T> {
    pub(crate) fn new(receiver: crossbeam::channel::Receiver<T>) -> Self {
        Self { receiver }
    }

    /// Wait for the task to complete and get the result
    pub fn wait(self) -> Result<T, TaskError> {
        self.receiver.recv().map_err(|_| TaskError::Cancelled)
    }

    /// Try to get the result without blocking
    pub fn try_get(&self) -> Option<Result<T, TaskError>> {
        match self.receiver.try_recv() {
            Ok(v) => Some(Ok(v)),
            Err(crossbeam::channel::TryRecvError::Empty) => None,
            Err(crossbeam::channel::TryRecvError::Disconnected) => Some(Err(TaskError::Cancelled)),
        }
    }

    /// Check if the task is complete
    pub fn is_complete(&self) -> bool {
        !self.receiver.is_empty()
    }
}

/// Task execution errors
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Task was cancelled")]
    Cancelled,

    #[error("Task panicked")]
    Panicked,

    #[error("Executor is shut down")]
    ExecutorShutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(|| 42);
        assert_eq!(task.priority, TaskPriority::Normal);
    }

    #[test]
    fn test_task_execution() {
        let task = Task::new(|| 42i32);
        let result = task.execute();
        let value = result.downcast::<i32>().unwrap();
        assert_eq!(*value, 42);
    }

    #[test]
    fn test_task_priority() {
        let low = Task::with_priority(|| (), TaskPriority::Low);
        let high = Task::with_priority(|| (), TaskPriority::High);
        assert!(low.priority < high.priority);
    }
}
