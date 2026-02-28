//! IOCP (I/O Completion Ports) backend for Windows.

#![cfg(target_os = "windows")]

use super::{Completion, Interest, IoHandle, Reactor, ReactorConfig};
use std::io;
use std::os::windows::io::RawHandle;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::IO::{
    CreateIoCompletionPort, GetQueuedCompletionStatusEx, OVERLAPPED_ENTRY,
};

/// Handle for IOCP registered resources.
#[derive(Debug, Clone)]
pub struct IocpHandle {
    /// User data for this handle.
    user_data: u64,
    /// Windows handle (stored as usize for Send+Sync).
    handle: usize,
}

// SAFETY: Windows handles are just integers and can be safely shared
unsafe impl Send for IocpHandle {}
unsafe impl Sync for IocpHandle {}

impl IoHandle for IocpHandle {
    fn user_data(&self) -> u64 {
        self.user_data
    }
}

impl IocpHandle {
    /// Get the raw handle.
    pub fn raw_handle(&self) -> RawHandle {
        self.handle as RawHandle
    }
}

/// IOCP reactor implementation.
pub struct IocpReactor {
    /// I/O completion port handle.
    iocp: HANDLE,
    /// Configuration.
    config: ReactorConfig,
    /// Next user_data value.
    next_user_data: AtomicU64,
}

impl IocpReactor {
    /// Associate a handle with the completion port.
    pub fn associate(&self, handle: RawHandle, user_data: u64) -> io::Result<()> {
        // SAFETY: CreateIoCompletionPort is safe to call with:
        // - A valid file/socket handle
        // - A valid IOCP handle (created in new())
        // - A completion key (user_data)
        // - 0 for NumberOfConcurrentThreads (use existing value)
        let result =
            unsafe { CreateIoCompletionPort(handle as HANDLE, self.iocp, user_data as usize, 0) };

        if result == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Queue an async file read operation.
    pub fn read_file(
        &self,
        _handle: &IocpHandle,
        _buffer: &mut [u8],
        _offset: u64,
    ) -> io::Result<()> {
        // In a real implementation, this would call ReadFile with OVERLAPPED
        Ok(())
    }

    /// Queue an async socket receive operation.
    pub fn recv_socket(&self, _handle: &IocpHandle, _buffer: &mut [u8]) -> io::Result<()> {
        // In a real implementation, this would call WSARecv with OVERLAPPED
        Ok(())
    }
}

impl Reactor for IocpReactor {
    type Handle = IocpHandle;

    fn new(config: ReactorConfig) -> io::Result<Self> {
        // SAFETY: CreateIoCompletionPort is safe to call with:
        // - INVALID_HANDLE_VALUE to create a new completion port
        // - 0 for ExistingCompletionPort (creating new)
        // - 0 for CompletionKey (not used when creating)
        // - concurrency_hint for NumberOfConcurrentThreads
        let iocp = unsafe {
            CreateIoCompletionPort(INVALID_HANDLE_VALUE, 0, 0, config.concurrency_hint as u32)
        };

        if iocp == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            iocp,
            config,
            next_user_data: AtomicU64::new(1),
        })
    }

    fn register(&self, handle: RawHandle, _interest: Interest) -> io::Result<Self::Handle> {
        let user_data = self.next_user_data.fetch_add(1, Ordering::Relaxed);
        self.associate(handle, user_data)?;
        Ok(IocpHandle {
            user_data,
            handle: handle as usize,
        })
    }

    fn submit(&self) -> io::Result<usize> {
        // IOCP doesn't have a submission queue - operations are submitted directly
        Ok(0)
    }

    fn wait(&self, timeout: Option<Duration>) -> io::Result<Vec<Completion>> {
        let timeout_ms = timeout.map(|d| d.as_millis() as u32).unwrap_or(u32::MAX);

        let mut entries = vec![
            OVERLAPPED_ENTRY {
                lpCompletionKey: 0,
                lpOverlapped: std::ptr::null_mut(),
                Internal: 0,
                dwNumberOfBytesTransferred: 0,
            };
            self.config.entries as usize
        ];

        let mut count = 0u32;

        // SAFETY: GetQueuedCompletionStatusEx is safe to call with:
        // - A valid IOCP handle (created in new())
        // - A valid pointer to an array of OVERLAPPED_ENTRY structs
        // - The correct count of entries in the array
        // - A pointer to receive the number of entries dequeued
        // - A timeout in milliseconds (u32::MAX for infinite)
        // - 0 for fAlertable (don't use alertable waits)
        let result = unsafe {
            GetQueuedCompletionStatusEx(
                self.iocp,
                entries.as_mut_ptr(),
                entries.len() as u32,
                &mut count,
                timeout_ms,
                0, // Don't alert
            )
        };

        if result == 0 {
            let err = io::Error::last_os_error();
            // WAIT_TIMEOUT is not an error
            if err.raw_os_error() == Some(258) {
                return Ok(Vec::new());
            }
            return Err(err);
        }

        let completions = entries[..count as usize]
            .iter()
            .map(|entry| Completion {
                user_data: entry.lpCompletionKey as u64,
                result: entry.dwNumberOfBytesTransferred as i32,
                flags: 0,
            })
            .collect();

        Ok(completions)
    }

    fn submit_and_wait(&self, _min_complete: usize) -> io::Result<Vec<Completion>> {
        self.wait(None)
    }
}

impl Drop for IocpReactor {
    fn drop(&mut self) {
        // SAFETY: CloseHandle is safe to call on any valid handle.
        // self.iocp was created in new() and is valid until this drop.
        // After this call, self.iocp is invalid and must not be used.
        unsafe {
            CloseHandle(self.iocp);
        }
    }
}
