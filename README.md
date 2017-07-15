# staticslot
Atomic pointer type for holding static variables.

## Overview
This crate provides a simple type, `StaticSlot<T>`, which is designed to make it easy to use static variables without much boilerplate or overhead. Usually you do not need any type of global variable, as it can introduce a number of problems into your code with bugs and testability. That being said, in certain applications a global variable is the most practical or efficient solution. This crate is targeted toward these uses.

A static slot is just a nullable pointer to some heap-allocated value with some extra features. We can declare one like this:

```rust
static MY_SLOT: StaticSlot<i32> = StaticSlot::NULL;
```

Here we're defining a static variable of type `StaticSlot<i32>` and initializing it to `StaticSlot::NULL`. In this state, our slot will start out "empty". To put an `i32` value into the slot we can use the `set()` method:

```rust
unsafe {
    MY_SLOT.set(42);
}
```

There are two things we can observe from this. First, we can set the value without having `MY_SLOT` be `static mut`. This is because the slot provides atomic, interior mutability for us. Secondly, calling `set()` is unsafe; this is because the compiler cannot guarantee we will free the memory for our `i32` when we are done with it.

If the value has been set, we can access it later using `get()`:

```rust
println!("{}", MY_SLOT.get().unwrap() + 100);
```

Since the slot may be empty, `get()` returns an `Option`. To clean up the memory when you are done, you can make the slot empty again by calling the `drop()` method. If you want to avoid unsafe code, you can put a dynamic lifetime on the value in the slot using the `with()` method, which introduces a scope for the value:

```rust
assert!(VALUE.get() == None);

MY_SLOT.with(42, || {
    // MY_SLOT contains 42 inside this block.
    assert!(MY_SLOT.get() == Some(&mut 42));
});

assert!(VALUE.get() == None);
```

If there is already a value in the slot, the previous value is restored at the end of the scope. Using `with()` guarantees that the memory for the value is cleaned up, and also allows you to nest calls with different values in the slot.

## Nightly compiler
Since the `StaticSlot` type is generic, the only way to initialize it statically is with constant functions, associated constants, or macros. Macros do not seem elegant for this use case, so currently this crate is using associated constants, which is not yet stabilized. Until it is stabilized, `staticslot` requires a nightly compiler to be used.

## License
MIT
