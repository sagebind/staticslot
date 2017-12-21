//! This crate provides a simple type, `StaticSlot<T>`, which is designed to make it easy to use static variables
//! without much boilerplate or overhead. Usually you do not need any type of global variable, as it can introduce a
//! number of problems into your code with bugs and testability. That being said, in certain applications a global
//! variable is the most practical or efficient solution. This crate is targeted toward these uses.
//!
//! # Usage
//!
//! A static slot is just a nullable pointer to some heap-allocated value with some extra features. We can declare one
//! like this:
//!
//! ```rust
//! use staticslot::StaticSlot;
//!
//! static MY_SLOT: StaticSlot<i32> = StaticSlot::NULL;
//! ```
//!
//! Here we're defining a static variable of type `StaticSlot<i32>` and initializing it to `StaticSlot::NULL`. In this
//! state, our slot will start out "empty". To put an `i32` value into the slot we can use the `set()` method:
//!
//! ```rust
//! use staticslot::StaticSlot;
//!
//! static MY_SLOT: StaticSlot<i32> = StaticSlot::NULL;
//!
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
//! use staticslot::StaticSlot;
//!
//! static MY_SLOT: StaticSlot<i32> = StaticSlot::NULL;
//!
//! unsafe {
//!     MY_SLOT.set(42);
//! }
//! println!("{}", *MY_SLOT.get().unwrap() + 100);
//! ```
//!
//! Since the slot may be empty, `get()` returns an `Option`. To clean up the memory when you are done, you can make the
//! slot empty again by calling the `drop()` method. If you want to avoid unsafe code, you can put a dynamic lifetime on
//! the value in the slot using the `with()` method, which introduces a scope for the value:
//!
//! ```rust
//! use staticslot::StaticSlot;
//!
//! static MY_SLOT: StaticSlot<i32> = StaticSlot::NULL;
//!
//! assert!(MY_SLOT.get() == None);
//! MY_SLOT.with(42, || {
//!     // MY_SLOT contains 42 inside this block.
//!     assert!(MY_SLOT.get() == Some(&mut 42));
//! });
//! assert!(MY_SLOT.get() == None);
//! ```
//!
//! If there is already a value in the slot, the previous value is restored at the end of the scope. Using `with()`
//! guarantees that the memory for the value is cleaned up, and also allows you to nest calls with different values in
//! the slot.
//!
//! # Unsized types
//!
//! Since `StaticSlot` depends on atomic operations, only `Sized` types can be stored in it, as unsized types would
//! require double-word atomics, which are not available on most architectures. It is possible to have an atomic unsized
//! pointer by having a double pointer, but that would harm the performance for the general case.
//!
//! If you need an unsized static slot (to hold a trait object, for example), you can simply put a `Box<T>` in the slot
//! to get the desired semantics. Below is an example of putting `Any` into a static slot.
//!
//! ```rust
//! use staticslot::StaticSlot;
//! use std::any::Any;
//! use std::sync::Mutex;
//!
//! static ANY: StaticSlot<Mutex<Box<Any + Send>>> = StaticSlot::NULL;
//!
//! let value = Mutex::new(Box::new(String::from("hello")) as Box<Any + Send>);
//!
//! ANY.with(value, || {
//!     if let Some(mutex) = ANY.get() {
//!         if let Some(string) = mutex.lock().unwrap().downcast_ref::<String>() {
//!             println!("It's a string({}): '{}'", string.len(), string);
//!         }
//!     }
//! });
//! ```
//!
//! This is useful when you need a singleton instance of some trait, but the implementation can vary.
use std::marker::PhantomData;
use std::sync::atomic::*;


/// A container for a statically owned value.
///
/// A slot can either hold a value or contain `NULL`. By default, a slot starts out `NULL` and can be populated with a
/// value later.
///
/// This container is meant to be used in conjunction with `static` variables for more controlled allocation and
/// de-allocation of shared instances. This type is unsafe because destructors are not guaranteed to be run at all, let
/// alone in the correct order. You *must* clean up your resources manually using the `drop()` method.
///
/// Think of it as an optimized `RefCell<Option<Box<T>>>` with atomic swapping and manual destruction.
pub struct StaticSlot<T> {
    /// Address to a heap-allocated value.
    address: AtomicUsize,
    _phantom: PhantomData<T>,
}

impl<T: 'static> Default for StaticSlot<T> {
    /// Create a new static slot initialized with `NULL`.
    fn default() -> Self {
        Self::NULL
    }
}

impl<T: 'static> StaticSlot<T> {
    /// A static slot with its value set to `NULL`. Useful for static initialization.
    pub const NULL: Self = Self {
        #[doc(hidden)]
        address: ATOMIC_USIZE_INIT,
        _phantom: PhantomData,
    };

    /// Create a new static slot that contains the given value.
    pub fn new(value: T) -> Self {
        let address = Box::into_raw(Box::new(value)) as usize;

        Self {
            address: AtomicUsize::new(address),
            _phantom: PhantomData,
        }
    }

    /// Check if the slot contains `NULL`.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.as_ptr().is_null()
    }

    /// Gets a reference to the value in the slot, if set.
    ///
    /// This method does not perform any initialization. For optimal performance, this performs a fast check if the
    /// slot is `NULL` and, if not, returns a reference.
    #[inline]
    pub fn get(&self) -> Option<&mut T> {
        let ptr = self.as_mut_ptr();

        if !ptr.is_null() {
            unsafe {
                Some(&mut *ptr)
            }
        } else {
            None
        }
    }

    /// Get a mutable reference to the value in the slot.
    ///
    /// If doing a null check every time you call `get()` is unnacceptable, then this unsafe variant will let you bypass
    /// that. Note that if the slot has not been initialized, the returned reference will be invalid and improper use
    /// could cause a segmentation fault.
    #[inline]
    pub unsafe fn get_unchecked(&self) -> &mut T {
        &mut *self.as_mut_ptr()
    }

    /// Returns an unsafe pointer to the contained value.
    ///
    /// If the slot is empty, will return a null pointer.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.address.load(Ordering::SeqCst) as *const _
    }

    /// Returns an unsafe mutable pointer to the contained value.
    ///
    /// If the slot is empty, will return a null pointer.
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.address.load(Ordering::SeqCst) as *mut _
    }

    /// Sets the static slot to a new value. If the slot was already set, the old value is dropped.
    ///
    /// This method is marked as unsafe because it can introduce memory leaks if `drop()` or `take()` is not manually
    /// called before the process exits.
    pub unsafe fn set(&self, value: T) {
        self.swap(Some(value));
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


#[cfg(test)]
mod tests {
    use super::StaticSlot;

    #[test]
    fn test_is_small() {
        use std::mem;

        assert!(mem::size_of::<StaticSlot<u64>>() == mem::size_of::<usize>());
    }

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
