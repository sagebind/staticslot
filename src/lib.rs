//! This crate provides a simple type, `StaticSlot<T>`, which is designed to make it easy to use static variables
//! without much boilerplate or overhead. Usually you do not need any type of global variable, as it can introduce a
//! number of problems into your code with bugs and testability. That being said, in certain applications a global
//! variable is the most practical or efficient solution. This crate is targeted toward these uses.
//!
//! A static slot is just a nullable pointer to some heap-allocated value with some extra features. We can declare one
//! like this:
//!
//! ```rust
//! static MY_SLOT: StaticSlot<i32> = StaticSlot::NULL;
//! ```
//!
//! Here we're defining a static variable of type `StaticSlot<i32>` and initializing it to `StaticSlot::NULL`. In this
//! state, our slot will start out "empty". To put an `i32` value into the slot we can use the `set()` method:
//!
//! ```rust
//! unsafe {
//!     MY_SLOT.set(42);
//! }
//! ```
//!
//! There are two things we can observe from this. First, we can set the value without having `MY_SLOT` be `static mut`.
//! This is because the slot provides atomic, interior mutability for us. Secondly, calling `set()` is unsafe; this is
//! because the compiler cannot guarantee we will free the memory for our `i32` when we are done with it.
//!
//! If the value has been set, we can access it later using `get()`:
//!
//! ```rust
//! println!("{}", MY_SLOT.get().unwrap() + 100);
//! ```
//!
//! Since the slot may be empty, `get()` returns an `Option`. To clean up the memory when you are done, you can make the
//! slot empty again by calling the `drop()` method. If you want to avoid unsafe code, you can put a dynamic lifetime on
//! the value in the slot using the `with()` method, which introduces a scope for the value:
//!
//! ```rust
//! assert!(VALUE.get() == None);
//!
//! MY_SLOT.with(42, || {
//!     // MY_SLOT contains 42 inside this block.
//!     assert!(MY_SLOT.get() == Some(&mut 42));
//! });
//!
//! assert!(VALUE.get() == None);
//! ```
//!
//! If there is already a value in the slot, the previous value is restored at the end of the scope. Using `with()`
//! guarantees that the memory for the value is cleaned up, and also allows you to nest calls with different values in
//! the slot.
#![feature(associated_consts)]
use std::marker::PhantomData;
use std::sync::atomic::*;


/// A container for a statically owned value.
///
/// This container is meant to be used in conjunction with `static` variables for more controlled allocation and
/// de-allocation of shared instances. This type is unsafe because destructors are not guaranteed to be run at all, let
/// alone in the correct order. You *must* clean up your resources manually using the `drop()` method.
///
/// Behaves kind of like a `RefCell<Option<Box<T>>>` with atomic swapping and manual destruction.
pub struct StaticSlot<T> {
    /// Address to a heap-allocated value.
    address: AtomicUsize,
    phantom: PhantomData<T>,
}

impl<T: 'static> StaticSlot<T> {
    /// A static slot with no value. Useful for static initialization.
    pub const NULL: Self = Self {
        address: ATOMIC_USIZE_INIT,
        phantom: PhantomData,
    };

    /// Create a new static slot with no value.
    pub fn new() -> Self {
        Self::NULL
    }

    /// Gets a reference to the value in the slot, if set.
    ///
    /// This method does not perform any initialization. For optimal performance, this peforms a fast check if the
    /// mantle is initialized and, if so, returns a pointer.
    #[inline]
    pub fn get(&self) -> Option<&mut T> {
        let address = self.address.load(Ordering::SeqCst);

        if address != 0 {
            unsafe {
                Some(&mut *(address as *mut _))
            }
        } else {
            None
        }
    }

    /// Invokes a closure, with the slot set to a given value.
    ///
    /// This method introduces a safe, controlled lifetime for the contained value. The value is shared for the duration
    /// of the execution of the closure. When the closure returns, the value is dropped.
    pub fn with<R, F: FnOnce() -> R>(&self, value: T, f: F) -> R {
        // Swap in the given value, and hold on to the previous.
        let previous = unsafe {
            self.swap(Some(value))
        };

        // Invoke the closure and save the return value.
        let result = f();

        // Now swap back in the previous value. The value passed in will be returned and dropped here.
        unsafe {
            self.swap(previous);
        }

        result
    }

    /// Sets the static slot to a new value. If the slot was already set, the old value is dropped.
    ///
    /// This method is marked as unsafe because it can introduce memory leaks if `drop()` or `take()` is not manually
    /// called before the process exits.
    pub unsafe fn set(&self, value: T) {
        self.swap(Some(value));
    }

    /// Takes the value out of the slot if it exists and frees any allocated heap memory.
    pub fn take(&self) -> Option<T> {
        unsafe {
            self.swap(None)
        }
    }

    /// Drops the value in the slot if any, and returns if a value was dropped.
    pub fn drop(&self) -> bool {
        unsafe {
            self.swap(None).is_some()
        }
    }

    /// Set the current value, returning the old value.
    unsafe fn swap(&self, value: Option<T>) -> Option<T> {
        // If a value is given, put it on the heap and get its address. Otherwise use null.
        let new_address = match value {
            Some(v) => Box::into_raw(Box::new(v)) as usize,
            None => 0,
        };

        // Swap in the new address and get the old address atomically.
        let old_address = self.address.swap(new_address, Ordering::SeqCst);

        // If the old address was not null, take the value off the heap and return it.
        if old_address != 0 {
            Some(*Box::from_raw(old_address as *mut _))
        } else {
            None
        }
    }
}

unsafe impl<T: Send> Send for StaticSlot<T> {}
unsafe impl<T: Sync> Sync for StaticSlot<T> {}


mod test {
    #[allow(unused_imports)]
    use super::StaticSlot;

    #[test]
    fn test_basic_usage() {
        static VALUE: StaticSlot<i32> = StaticSlot::NULL;

        assert!(VALUE.get() == None);
        unsafe {
            VALUE.set(1);
        }
        assert!(VALUE.get() == Some(&mut 1));
        VALUE.drop();
        assert!(VALUE.get() == None);
    }

    #[test]
    fn test_with() {
        static VALUE: StaticSlot<i32> = StaticSlot::NULL;

        assert!(VALUE.get() == None);

        VALUE.with(1, || {
            assert!(VALUE.get() == Some(&mut 1));

            VALUE.with(2, || {
                assert!(VALUE.get() == Some(&mut 2));
            });

            assert!(VALUE.get() == Some(&mut 1));
        });

        assert!(VALUE.get() == None);
    }
}
