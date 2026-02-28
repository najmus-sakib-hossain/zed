//! I/O interest flags.

use bitflags::bitflags;

bitflags! {
    /// Interest flags for I/O event registration.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Interest: u8 {
        /// Interest in read events.
        const READABLE = 0b0000_0001;

        /// Interest in write events.
        const WRITABLE = 0b0000_0010;

        /// Interest in error events.
        const ERROR = 0b0000_0100;

        /// Interest in hangup events.
        const HUP = 0b0000_1000;

        /// Edge-triggered mode (vs level-triggered).
        const EDGE = 0b0001_0000;

        /// One-shot mode (auto-deregister after event).
        const ONESHOT = 0b0010_0000;
    }
}

impl Interest {
    /// Create interest for read events only.
    pub fn readable() -> Self {
        Self::READABLE
    }

    /// Create interest for write events only.
    pub fn writable() -> Self {
        Self::WRITABLE
    }

    /// Create interest for both read and write events.
    pub fn both() -> Self {
        Self::READABLE | Self::WRITABLE
    }

    /// Add edge-triggered mode.
    pub fn edge(self) -> Self {
        self | Self::EDGE
    }

    /// Add one-shot mode.
    pub fn oneshot(self) -> Self {
        self | Self::ONESHOT
    }

    /// Check if interested in read events.
    pub fn is_readable(&self) -> bool {
        self.contains(Self::READABLE)
    }

    /// Check if interested in write events.
    pub fn is_writable(&self) -> bool {
        self.contains(Self::WRITABLE)
    }

    /// Check if edge-triggered mode is enabled.
    pub fn is_edge(&self) -> bool {
        self.contains(Self::EDGE)
    }

    /// Check if one-shot mode is enabled.
    pub fn is_oneshot(&self) -> bool {
        self.contains(Self::ONESHOT)
    }
}

impl Default for Interest {
    fn default() -> Self {
        Self::READABLE
    }
}
