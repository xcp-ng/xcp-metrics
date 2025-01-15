//! Hypercall handling
//!
//! Abstractions over the mechanism by which hypercalls are sent.
//!
//! If hosted: `/dev/xen/privcmd` is used via `ioctl()`.
//! If freestanding: A direct hypercall is issued.

use std::error::Error;

pub mod unix;

/// Wrapper of a reference into a hypercall-safe buffer.
pub trait XenConstBuffer<T> {
    /// Get a hypercall-safe reference to underlying data.
    fn as_hypercall_ptr(&self) -> *const T;
}

/// Wrapper of a mutable reference into a mutable hypercall-safe buffer.
pub trait XenMutBuffer<T> {
    /// Get a hypercall-safe mutable reference to underlying data.
    fn as_hypercall_ptr(&mut self) -> *mut T;

    /// Update original reference with new data.
    unsafe fn update(&mut self);
}

pub trait XenHypercall: Sized {
    type Error: Error + Send + Sync + 'static;

    unsafe fn hypercall5(&self, cmd: usize, param: [usize; 5]) -> usize;

    unsafe fn hypercall4(&self, cmd: usize, param: [usize; 4]) -> usize {
        self.hypercall5(cmd, [param[0], param[1], param[2], param[3], 0])
    }

    unsafe fn hypercall3(&self, cmd: usize, param: [usize; 3]) -> usize {
        self.hypercall4(cmd, [param[0], param[1], param[2], 0])
    }

    unsafe fn hypercall2(&self, cmd: usize, param: [usize; 2]) -> usize {
        self.hypercall3(cmd, [param[0], param[1], 0])
    }

    unsafe fn hypercall1(&self, cmd: usize, param: usize) -> usize {
        self.hypercall2(cmd, [param, 0])
    }

    unsafe fn hypercall0(&self, cmd: usize) -> usize {
        self.hypercall1(cmd, 0)
    }

    fn make_const_object<T: Copy>(&self, buffer: &T)
        -> Result<impl XenConstBuffer<T>, Self::Error>;

    fn make_mut_buffer<T: Copy>(&self, buffer: &mut T)
        -> Result<impl XenMutBuffer<T>, Self::Error>;

    // Slices needs some special handling as they are not Copy themselves
    // and a pointer to a slice doesn't point to its first element.

    fn make_const_slice<T: Copy>(&self, slice: &[T])
        -> Result<impl XenConstBuffer<T>, Self::Error>;

    fn make_mut_slice<T: Copy>(&self, slice: &mut [T])
        -> Result<impl XenMutBuffer<T>, Self::Error>;
}
