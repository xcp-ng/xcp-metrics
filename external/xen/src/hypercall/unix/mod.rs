//! UNIX Xen interface
//!
//! Implementation of [`XenInterface`] for UNIX-like systems. Ought to work on both Linux
//! and *BSD systems implementing `/dev/xen/privcmd`.
//!

pub mod buffer;

use core::{marker::PhantomData, ptr::addr_of_mut};
use std::{fs::File, io, os::fd::AsRawFd};

use buffer::{UnixConstXenBuffer, UnixConstXenSlice, UnixMutXenBuffer, UnixMutXenSlice};
use nix::errno::Errno;

use super::{XenConstBuffer, XenHypercall, XenMutBuffer};

/// An abstraction over Xen privcmd device.
#[derive(Debug)]
pub struct UnixXenHypercall {
    privcmd_device: File,
    hypercall_device: File,
}

#[cfg(target_os = "linux")]
type PrivCmdField = u64;

#[cfg(not(target_os = "linux"))]
type PrivCmdField = usize;

#[repr(C)]
#[derive(Debug)]
struct PrivCmdArg {
    /// Identifier for the issued hypercall type
    op: PrivCmdField,
    /// Hypercall-specific arguments
    args: [PrivCmdField; 5],
    #[cfg(not(target_os = "linux"))]
    /// Return code of the `ioctl` in *BSD systems
    ret: PrivCmdField,
}

mod ioctl {
    nix::ioctl_write_ptr_bad!(
        hypercall,
        nix::ioc!(0, b'P', 0, core::mem::size_of::<super::PrivCmdArg>()),
        super::PrivCmdArg
    );
}

/// Path to the `privcmd` device in a hosted environment.
const PATH_PRIVCMD: &str = "/dev/xen/privcmd";

/// Path to `hypercall` device in a hosted environment.
const PATH_HYPERCALL: &str = "/dev/xen/hypercall";

impl UnixXenHypercall {
    pub fn new() -> Result<Self, io::Error> {
        Ok(Self {
            privcmd_device: File::options()
                .read(true)
                .write(true)
                .open(PATH_PRIVCMD)?,
            hypercall_device: File::options()
                .read(true)
                .write(true)
                .open(PATH_HYPERCALL)?,
        })
    }
}

impl XenHypercall for UnixXenHypercall {
    type Error = Errno;

    unsafe fn hypercall5(&self, cmd: usize, param: [usize; 5]) -> usize {
        let mut privcmd_arg = PrivCmdArg {
            op: cmd as _,
            args: [
                param[0] as _,
                param[1] as _,
                param[2] as _,
                param[3] as _,
                param[4] as _,
            ],
            #[cfg(not(target_os = "linux"))]
            ret: 0,
        };

        match ioctl::hypercall(self.privcmd_device.as_raw_fd(), addr_of_mut!(privcmd_arg)) {
            Ok(ret) => ret as _,
            Err(ret) => ret as _, // ugh
        }
    }

    fn make_const_object<'a, T: Copy + ?Sized>(
        &self,
        buffer: &'a T,
    ) -> Result<impl XenConstBuffer<T>, Self::Error> {
        let mut call_buffer = self.alloc_xencall_buffer()?;
        call_buffer.write(*buffer);

        Ok(UnixConstXenBuffer {
            original: PhantomData::<&'a T>,
            buffer: call_buffer,
        })
    }

    fn make_mut_buffer<T: Copy + ?Sized>(
        &'_ self,
        buffer: &mut T,
    ) -> Result<impl XenMutBuffer<T>, Self::Error> {
        let mut call_buffer = self.alloc_xencall_buffer()?;
        call_buffer.write(*buffer);

        Ok(UnixMutXenBuffer {
            original: buffer,
            buffer: call_buffer,
        })
    }

    fn make_const_slice<'a, T: Copy + Sized>(
        &self,
        slice: &'a [T],
    ) -> Result<impl XenConstBuffer<T>, Self::Error> {
        let mut call_buffer = self.alloc_xencall_slice(slice.len())?;
        call_buffer.copy_from_slice(slice);

        Ok(UnixConstXenSlice {
            original: PhantomData::<&'a [T]>,
            buffer: call_buffer,
        })
    }

    fn make_mut_slice<T: Copy + Sized>(
        &self,
        slice: &mut [T],
    ) -> Result<impl XenMutBuffer<T>, Self::Error> {
        let mut call_buffer = self.alloc_xencall_slice(slice.len())?;
        call_buffer.copy_from_slice(slice);

        Ok(UnixMutXenSlice {
            original: slice,
            buffer: call_buffer,
        })
    }
}
