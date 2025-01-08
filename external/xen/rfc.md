# Rust 'xen' crate/library interface design

See [1] for more context.

This RFC proposes parts of the 'xen' crate interface that would directly or indirectly
(through internal wrappers) be used by users.
Those users could be a userland toolstack, a unikernel application (e.g XTF, Unikraft),
some other freestanding environment (e.g OVMF, Linux, Redox OS), ...

These users can have a very different execution environment, this crate aims to provide
a uniform interface while allowing flexibility for exposing platform-specific bits.

## Design philosophy

This crate should feel natural for a Rust developper, thus, any Rust developper with some
understanding on the Xen hypercall operations should be able to use this crate.
Moreover, we want this crate to be maintainable and feel "idiomatic", and not introduce
confusing behavior onto the user. Note that this crate will heavily use unsafe code.

Some principles are proposed :

Use or provide idiomatic abstractions when relevant (reuse standard traits).

Examples:
  Provide (optional) Future<...> abstractions for event channels
  Provide a iterator-based (Stream ? [2]) abstraction for guest console.

Don't restrict features to some execution environment, use modular abstractions (e.g traits)
to allow the user to specify the missing bits himself / provide its own implementation.
Note that it doesn't prevent us from exposing the platform-specific bits onto the
types themselves (e.g UnixXenEventChannel can still expose its file descriptor).

Example:
  If we provide a event channel abstraction based on hypercall but it doesn't implement Future<...>,
  the user can still implement its own type on top of the hypercall implementation, and
  use its own async runtime (e.g based on interrupts) to implement Future<...> himself.
  There could be 2 traits for varying needs :
    EventChannel (base trait) and AsyncEventChannel (await-able EventChannel)

  We can have both RawEventChannel based on XenHypercall that only implements EventChannel
  and another type TokioEventChannel that provides both EventChannel and AsyncEventChannel
  and integrates with tokio runtime.

Safe wrappers must be "sound" and unsafe code shall not cause undefined behavior.
- safe wrappers must not cause undefined behavior on their own
- unsafe code should not cause undefined behavior if properly used

This is a bit tricky due to some Xen specificities, but we expect hypercall to be well
formed (we can add validation tools for that) including have its parameter indirectly
respect the aliasing rules [3].
Although, we assume that Xen is well-behaving regarding its ABI.
We don't define misuse of a hypercall that can harm the guest himself, but we care
about not causing a undefined behavior (e.g by passing a buggy pointer) through the
hypercall interface that can overwrite unrelated/arbitrary kernel memory.

## Hypercall interface

We introduce a XenHypercall trait that exposes a raw primitive for making hypercalls.
This interface supposes nothing on the ABI used in Xen, and its the responsibility
of the user of such interface (often safe wrappers) that the hypercall made is
meaningful.

This interface is mostly to only be used by the crate developpers to build safe
wrappers on top, or by advanced user for using non-exposed/WIP hypercall interfaces
or bypassing the safe wrappers.

We can implement this interface for freestanding platforms using direct native hypercalls
(e.g using inline assembly) for freestanding platforms or in userland using special devices
like privcmd/xencall on most Unix platforms.

```rust
pub trait XenHypercall: Sized {
    unsafe fn hypercall5(&self, cmd: usize, param: [usize; 5]) -> usize;

    unsafe fn hypercall4(&self, cmd: usize, param: [usize; 4]) -> usize;
    unsafe fn hypercall3(&self, cmd: usize, param: [usize; 3]) -> usize;
    unsafe fn hypercall2(&self, cmd: usize, param: [usize; 2]) -> usize;
    unsafe fn hypercall1(&self, cmd: usize, param: usize) -> usize;
    unsafe fn hypercall0(&self, cmd: usize) -> usize;

    /* ... */
}
```

### Hypercall-safe buffers

One difficulty is that in a freestanding environment, we need to use pointers to
original data. But in a hosted environment, we need to make special buffers instead
for that.

We introduce the Xen{Const/Mut}Buffer generic trait that wraps a reference in a
"hypercall-safe" buffer that may or may not be a bounce buffer.

```rust
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

// The user will make those wrappers using dedicated functions in XenHypercall trait.

trait XenHypercall: Sized {
    /* ... */
    type Error;

    fn make_const_object<T: Copy + ?Sized>(
        &self,
        buffer: &T,
    ) -> Result<impl XenConstBuffer<T>, Self::Error>;

    fn make_mut_buffer<T: Copy + ?Sized>(
        &self,
        buffer: &mut T,
    ) -> Result<impl XenMutBuffer<T>, Self::Error>;

    fn make_const_slice<T: Copy + Sized>(
        &self,
        slice: &[T],
    ) -> Result<impl XenConstBuffer<T>, Self::Error>;

    fn make_mut_slice<T: Copy + Sized>(
        &self,
        slice: &mut [T],
    ) -> Result<impl XenMutBuffer<T>, Self::Error>;
}
```

Example use:

```rust
fn demo_hypercall<H: XenHypercall>(interface: &H, buffer: &mut [u8]) -> Result<(), H::Error> {
    let buffer_size = buffer.len();
    // make a hypercall-safe wrapper of `buffer`
    let hyp_buffer = interface.make_mut_slice(buffer)?;

    let op = SomeHypercallStruct {
        buffer: hyp_buffer.as_hypercall_ptr(),
        buffer_size: buffer_size as _,
    };
    // Do the same for hyp_op
    let hyp_op = interface.make_const_object(&op)?;

    unsafe {
        interface.hypercall1(SOME_CMD, hyp_op.as_hypercall_ptr().addr());
        // assume success
        hyp_buffer.update(); // update buffer back
    }

    Ok(())
}
```

Note that freestanding case, we can use a thin zero-copy wrapper :
```rust
/// Constant xen buffer that passes the reference as-is.
pub(super) struct DirectConstXenBuffer<'a, T>(&'a T);

impl<T> XenConstBuffer<T> for DirectConstXenBuffer<'_, T> {
    fn as_hypercall_ptr(&self) -> *const T {
        self.0
    }
}
// ...
```

TODO:
Do we need to clarify the lifetimes (e.g should trait indicate a lifetime binding with
original data) ? Try to answer with RPITIT and Rust 2024 capture rules [4].

Try to unify make_const_object and make_const_slice (along with mut variant). `*const [T]`
is a bit more subtle to create and we cannot trivially cast a address into a pointer and
need to use special functions for that (`core::ptr::slice_from_raw_parts` ?).
But for that, we need to know that T is actually a slice before using this function.

## Event channels

TODO

[1] - Interfacing Rust with Xen - Alejandro Vallejo, XenServer BU, Cloud Software Group
https://youtu.be/iFh4n2kbAwM

[2] - The Stream Trait
https://rust-lang.github.io/async-book/05_streams/01_chapter.html

[3] - Aliasing
https://doc.rust-lang.org/nomicon/aliasing.html

[4] - https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html
https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html
