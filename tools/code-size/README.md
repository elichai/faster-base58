# Linked size probe

`scripts/code_size.py` creates a standalone Cargo package from these three source
files. Each build selects exactly one backend and encode, decode, or both. The
backend and direction are compiler cfgs on the probe only, not library features.
All named backend selectors and forced-kernel controls enable the library's
`__internals` feature. Auto and no-allocation probes use the normal public API.
`--cache` selects the optional `cache` feature for a separate cohort; the default
has no process-wide table cache. Competitors keep their own dependency features.
The package uses the checked-out library by path. Its optional Malachite dependency
is enabled only for the external comparison adapter.

Native executables consume one dynamic OS argument, allocate output, and pass the
result through `black_box`. Native shared libraries and Wasm modules expose dynamic
pointer/length conversion functions, allocation/free, and repeated-call functions.
The empty baseline has the same scaffold and no conversion call. It therefore
accounts for the allocator, panic/runtime support, executable argument handling,
linker metadata, and repeat-loop ABI that survive each particular link.

`size_alloc(n)` returns zero-initialized storage. Free it exactly once with
`size_dealloc(ptr, n)`. Input and output pointers to a conversion must be valid,
non-overlapping allocations for their complete lengths, even at zero length.
Conversion failures return `usize::MAX`; decoding validates UTF-8. Repeat helpers
return the final length; zero iterations return zero. These helpers are measurement
infrastructure, not a public library API.

The build uses release optimization, fat LTO, 16 codegen units, no debug info, and
the target's default panic strategy. The script overrides ambient Rust flags with
the recorded CPU flag, preserves unstripped and stripped artifacts, and records
commands and compiler identity. It never reports an `.rlib` as a deployed program.
