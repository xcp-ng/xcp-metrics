//! Rust bindings for communicating with the Xen hypervisor
//!
//! This crate is meant to be used in any of the following configurations:
//!   * hosted: `std`, and hypercalls mediated through `/dev/xen/privcmd`.
//!   * freestanding: `no_std` and direct hypercalls. Meant for unikernels.

pub mod hypercall;
pub mod sysctl;
pub mod domctl;
pub mod abi;

/// Abstraction of a domain ID. This is the number used by Xen to identify a
/// single domain at runtime.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct DomId(pub u16);

/// Wrapper for pointers. This type is intended to be used with pointers in
/// the hypercall buffers. On 32bit machines we still want pointers aligned to
/// 64bit boundaries with 64bit sizes (otherwise the hypervisor and its guests
/// could clash).
#[repr(align(8))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Align64<T>(pub T);

impl<T> Default for Align64<T> {
    fn default() -> Self {
        // This is required because `*mut U` can't implement Default. We take the convention that
        // `Default` means "zero". For `t: Align64<*mut T>` that means `t` is null.
        unsafe { Self(core::mem::zeroed()) }
    }
}