# staticslot
Atomic pointer type for holding static variables.

[![Build Status](https://semaphoreci.com/api/v1/sagebind/staticslot/branches/master/badge.svg)](https://semaphoreci.com/sagebind/staticslot)
[![Crates.io](https://img.shields.io/crates/v/staticslot.svg)](https://crates.io/crates/staticslot)
[![Documentation](https://docs.rs/staticslot/badge.svg)](https://docs.rs/staticslot)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

[Documentation](https://docs.rs/staticslot)

## Installation
Add this to your `Cargo.toml` file:

```rust
[dependencies]
staticslot = "0.2"
```

## Overview
This crate provides a simple type, `StaticSlot<T>`, which is designed to make it easy to use static variables without much boilerplate or overhead. Usually you do not need any type of global variable, as it can introduce a number of problems into your code with bugs and testability. That being said, in certain applications a global variable is the most practical or efficient solution. This crate is targeted toward these uses.

A static slot is just a nullable pointer to some heap-allocated value with some extra features. We can declare one like this:

```rust
static GLOBAL_STRINGS: StaticSlot<Vec<String>> = StaticSlot::NULL;
```

Then we can `get()` and `set()` the value throughout our program. In addition, a number of convenience methods are also provided. See the documentation for details about semantics and safety.

## License
MIT
