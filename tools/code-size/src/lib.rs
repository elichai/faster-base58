#![allow(unexpected_cfgs)]

#[cfg(any(size_direction = "encode", size_direction = "decode", size_direction = "both"))]
mod entry;

#[cfg(size_count_allocations)]
mod allocations;
