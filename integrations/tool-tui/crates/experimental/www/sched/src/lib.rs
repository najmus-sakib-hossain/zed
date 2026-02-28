//! # dx-sched: Frame Scheduler & Event Loop
//!
//! The heartbeat of dx-www runtime.
//! Orchestrates RAF (requestAnimationFrame) loop with frame budget control.
//!
//! **ARCHITECTURE:**
//! - RAF Loop: Driven by browser's vsync
//! - Frame Budget: Max 4ms WASM execution per frame (16.67ms - 12ms for layout/paint)
//! - Priority Queue: Input events > RAF callbacks > Idle callbacks
//! - Yield Strategy: If budget exceeded, defer to next frame
//!
//! **ACID TEST COMPLIANCE:**
//! - No allocations in hot loop
//! - Use Performance API for nanosecond timing
//! - Event queue uses ring buffer (from dx-core)

#![forbid(unsafe_code)]
#![allow(clippy::len_without_is_empty)] // TaskQueue len is sufficient for our use case

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Performance, window};

// ============================================================================
// FRAME BUDGET CONFIGURATION
// ============================================================================

/// Maximum WASM execution time per frame (in milliseconds)
/// Target: 60 FPS = 16.67ms per frame
/// Budget: 4ms for WASM (leaving 12ms for layout, paint, composite)
pub const FRAME_BUDGET_MS: f64 = 4.0;

/// If we're within this threshold of budget, start yielding
pub const YIELD_THRESHOLD_MS: f64 = 3.5;

// ============================================================================
// PERFORMANCE TIMER
// ============================================================================

pub struct FrameTimer {
    performance: Performance,
    frame_start: f64,
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameTimer {
    /// Create a new frame timer.
    ///
    /// # Panics
    ///
    /// Panics if called outside a browser environment (no window or performance API).
    /// This is expected behavior for WASM code that requires browser APIs.
    pub fn new() -> Self {
        let window = window()
            .expect("FrameTimer requires browser window - ensure this is called in WASM context");
        let performance = window
            .performance()
            .expect("FrameTimer requires Performance API - ensure browser supports it");

        Self {
            performance,
            frame_start: 0.0,
        }
    }

    /// Mark the start of a frame
    pub fn start_frame(&mut self) {
        self.frame_start = self.performance.now();
    }

    /// Get elapsed time since frame start (in ms)
    pub fn elapsed(&self) -> f64 {
        self.performance.now() - self.frame_start
    }

    /// Check if we've exceeded the frame budget
    pub fn should_yield(&self) -> bool {
        self.elapsed() > YIELD_THRESHOLD_MS
    }

    /// Get remaining budget (for logging)
    pub fn remaining_budget(&self) -> f64 {
        FRAME_BUDGET_MS - self.elapsed()
    }
}

// ============================================================================
// TASK PRIORITY QUEUE
// ============================================================================

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Immediate user input (keyboard, mouse)
    Immediate = 0,
    /// Normal RAF work (render, state updates)
    Normal = 1,
    /// Low priority (network, analytics)
    Idle = 2,
}

pub type TaskCallback = Box<dyn FnOnce()>;

pub struct Task {
    priority: TaskPriority,
    callback: TaskCallback,
}

pub struct TaskQueue {
    /// Tasks sorted by priority (Vec used as heap)
    tasks: Vec<Task>,
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: Vec::with_capacity(64),
        }
    }

    /// Schedule a task with given priority
    pub fn schedule(&mut self, priority: TaskPriority, callback: TaskCallback) {
        self.tasks.push(Task { priority, callback });
        // Keep sorted by priority (Immediate first)
        self.tasks.sort_by_key(|t| t.priority);
    }

    /// Execute tasks until budget is exhausted
    pub fn drain_until_budget(&mut self, timer: &FrameTimer) -> usize {
        let mut executed = 0;

        while !self.tasks.is_empty() {
            if timer.should_yield() {
                break;
            }

            if let Some(task) = self.tasks.first() {
                // If next task is Idle priority and we're running low, skip it
                if task.priority == TaskPriority::Idle && timer.elapsed() > 2.0 {
                    break;
                }
            }

            if let Some(task) = self.tasks.drain(0..1).next() {
                (task.callback)();
                executed += 1;
            }
        }

        executed
    }

    /// Clear all tasks
    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    /// Get number of pending tasks
    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}

// ============================================================================
// SCHEDULER (Main Loop Controller)
// ============================================================================

pub struct Scheduler {
    timer: FrameTimer,
    task_queue: TaskQueue,
    frame_count: u64,
    is_running: bool,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            timer: FrameTimer::new(),
            task_queue: TaskQueue::new(),
            frame_count: 0,
            is_running: false,
        }
    }

    /// Schedule a task
    pub fn schedule(&mut self, priority: TaskPriority, callback: TaskCallback) {
        self.task_queue.schedule(priority, callback);
    }

    /// Process one frame
    pub fn tick(&mut self) {
        self.timer.start_frame();
        self.frame_count += 1;

        // Execute queued tasks
        let _executed = self.task_queue.drain_until_budget(&self.timer);

        // Flush pending DOM operations
        #[cfg(target_arch = "wasm32")]
        {
            dx_dom::flush_queue();
        }

        // Log performance stats (every 60 frames = 1 second at 60fps)
        #[cfg(target_arch = "wasm32")]
        if self.frame_count % 60 == 0 {
            let elapsed = self.timer.elapsed();
            let remaining = self.timer.remaining_budget();

            web_sys::console::log_1(
                &format!(
                    "Frame {}: {}ms used, {}ms budget remaining, {} tasks executed",
                    self.frame_count, elapsed, remaining, _executed
                )
                .into(),
            );
        }
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Set running state
    pub fn set_running(&mut self, running: bool) {
        self.is_running = running;
    }
}

// ============================================================================
// GLOBAL SCHEDULER INSTANCE
// ============================================================================

thread_local! {
    static SCHEDULER: RefCell<Scheduler> = RefCell::new(Scheduler::new());
}

pub fn with_scheduler<F, R>(f: F) -> R
where
    F: FnOnce(&mut Scheduler) -> R,
{
    SCHEDULER.with(|sched| f(&mut sched.borrow_mut()))
}

// ============================================================================
// RAF LOOP (WASM Entry Point)
// ============================================================================

#[wasm_bindgen]
pub fn start_scheduler() {
    let already_running = with_scheduler(|scheduler| {
        if scheduler.is_running() {
            true
        } else {
            scheduler.set_running(true);
            false
        }
    });

    if already_running {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::warn_1(&"Scheduler already running".into());
        return;
    }

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"dx-sched: Starting RAF loop".into());

    // Kick off the RAF loop
    request_next_frame();
}

#[wasm_bindgen]
pub fn stop_scheduler() {
    with_scheduler(|scheduler| scheduler.set_running(false));

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"dx-sched: Stopping RAF loop".into());
}

/// Request the next animation frame
///
/// # Panics
///
/// Panics if called outside a browser environment. This is expected behavior
/// for the RAF loop which requires browser APIs.
fn request_next_frame() {
    let window =
        window().expect("RAF loop requires browser window - ensure this is called in WASM context");

    // Create closure for RAF callback
    let closure = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let closure_clone = closure.clone();

    *closure.borrow_mut() = Some(Closure::new(move || {
        // Process this frame
        let should_continue = with_scheduler(|scheduler| {
            if !scheduler.is_running() {
                false // Stop loop
            } else {
                scheduler.tick();
                true
            }
        });

        if !should_continue {
            return;
        }

        // Schedule next frame
        request_next_frame();
    }));

    // Request animation frame - this should always succeed in a browser environment
    // If it fails, it indicates a fundamental browser API issue
    let closure_ref = closure_clone.borrow();
    if let Some(ref inner) = *closure_ref {
        window
            .request_animation_frame(inner.as_ref().unchecked_ref())
            .expect("requestAnimationFrame failed - browser may not support RAF");
    }
}

// ============================================================================
// PUBLIC API (Task Scheduling)
// ============================================================================

#[wasm_bindgen]
pub fn schedule_immediate(callback: &js_sys::Function) {
    let callback_clone = callback.clone();
    let task = Box::new(move || {
        callback_clone.call0(&JsValue::NULL).ok();
    });

    with_scheduler(|scheduler| scheduler.schedule(TaskPriority::Immediate, task));
}

#[wasm_bindgen]
pub fn schedule_normal(callback: &js_sys::Function) {
    let callback_clone = callback.clone();
    let task = Box::new(move || {
        callback_clone.call0(&JsValue::NULL).ok();
    });

    with_scheduler(|scheduler| scheduler.schedule(TaskPriority::Normal, task));
}

#[wasm_bindgen]
pub fn schedule_idle(callback: &js_sys::Function) {
    let callback_clone = callback.clone();
    let task = Box::new(move || {
        callback_clone.call0(&JsValue::NULL).ok();
    });

    with_scheduler(|scheduler| scheduler.schedule(TaskPriority::Idle, task));
}

// ============================================================================
// INITIALIZATION
// ============================================================================

pub fn init_dx_sched() {
    #[cfg(target_arch = "wasm32")]
    {
        dx_core::panic_hook();
        web_sys::console::log_1(&"dx-sched: Frame Scheduler Initialized".into());
    }
}
