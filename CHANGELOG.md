0.32.0 (2026-02-04)
===

## What's Changed
* Update to MMTk core PR #1308 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/238
* Fix the scan for module usings field: use 4 as the step instead of 3 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/281
* Update the Rust binding for upstream changes by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/284
* Move to Rust 1.92 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/287
* Bump version to 0.31.2 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/289

**Full Changelog**: https://github.com/mmtk/mmtk-julia/compare/v0.31.0...v0.32.0

0.31.0 (2025-04-17)
===

## What's Changed
* Fixing the binding to apply the changes from Julia's PR #57625 by @udesou in https://github.com/mmtk/mmtk-julia/pull/233
* Update mmtk-core to v0.31.0

**Full Changelog**: https://github.com/mmtk/mmtk-julia/compare/v0.30.5...v0.31.0

0.30.0 (2024-12-20)
===

## What's Changed
* Update mmtk-core to v0.30.0

**Full Changelog**: https://github.com/mmtk/mmtk-julia/compare/v0.29.0..v0.30.0


0.29.0 (2024-12-18)
===

The initial release for MMTk Julia. It includes support for a non-moving Immix plan.
The version follows MMTk projects convention that the binding uses the same version as mmtk-core.

## What's Changed
* Fixing run pending finalizers by @udesou in https://github.com/mmtk/mmtk-julia/pull/14
* Fix/segfault rai by @udesou in https://github.com/mmtk/mmtk-julia/pull/16
* CI GitHub action (#13) by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/23
* Update/julia master by @udesou in https://github.com/mmtk/mmtk-julia/pull/18
* Update Julia by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/24
* Allow MMTk core to test Julia binding by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/25
* Set enum-map >= 2.1 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/27
* Remove some warnings when compiling Julia by @kpamnany in https://github.com/mmtk/mmtk-julia/pull/29
* Use immix_no_nursery_copy. Update rust toolchain by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/31
* Remove some code by @kpamnany in https://github.com/mmtk/mmtk-julia/pull/37
* Update to mmtk-core PR #781 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/40
* Update README.md by @NHDaly in https://github.com/mmtk/mmtk-julia/pull/47
* Treat task.gcstacks as roots by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/52
* Update Julia to use MMTk for perm alloc by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/51
* Expose object reference write by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/55
* Implement JuliaMemorySlice, and expose mmtk_memory_region_copy by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/53
* Use MMTK VM space by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/56
* Support sticky immix by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/48
* Update to MMTk core PR #817 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/58
* Update to mmtk-core PR #838 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/61
* Update Julia upstream 909c57f by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/60
* Update the total (gc) time in Julia's gc_num by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/62
* Fix the argument type in get_lo_size by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/32
* Rename ambiguous `scan_thread_root{,s}` functions by @k-sareen in https://github.com/mmtk/mmtk-julia/pull/63
* Embed mutator in _jl_tls_states_t by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/64
* Assert alloc size alignment by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/68
* Simplify process_edge by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/72
* Add an assertion in process_edge to make sure objects are in MMTk heap. by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/75
* Update README by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/76
* Remove counted malloc by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/80
* Fix the wrong julia version when PR#80 was merged. by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/81
* Use Julia's finalizer implementation by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/78
* Use cheap safepoint in alloc by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/83
* Inline runtime alloc by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/79
* Rewriting C code for scan_object, get_size, and get_object_start_ref in Rust by @udesou in https://github.com/mmtk/mmtk-julia/pull/82
* Update ci-test.sh by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/85
* Remove the statically linked C code by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/87
* Update to MMTk core PR #875 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/86
* Making sure worker_tls is different than the mutator tls by @udesou in https://github.com/mmtk/mmtk-julia/pull/90
* Remove global ROOT_NODES/EDGES (merge after #86) by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/89
* Fixing mismatch between C and Rust version of Julia functions by @udesou in https://github.com/mmtk/mmtk-julia/pull/92
* Improving CI tests by @udesou in https://github.com/mmtk/mmtk-julia/pull/95
* Update Julia upstream 43bf2c8 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/71
* Setting up a small set of tests to run from mmtk-core's CI by @udesou in https://github.com/mmtk/mmtk-julia/pull/97
* Splitting LinearAlgebra tests  by @udesou in https://github.com/mmtk/mmtk-julia/pull/103
* Updating code to reflect API change; Bumping rust-toolchain to 1.71.1 by @udesou in https://github.com/mmtk/mmtk-julia/pull/99
* Refactoring block_for_gc by @udesou in https://github.com/mmtk/mmtk-julia/pull/100
* Remove code that iterates over bindings table by @udesou in https://github.com/mmtk/mmtk-julia/pull/91
* Update README.md by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/105
* Update to MMTk core PR #949 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/109
* Run CI and mergify for v1.9.2+RAI by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/112
* Updating to latest version of mmtk-core by @udesou in https://github.com/mmtk/mmtk-julia/pull/111
* Escape + in the branch name for the workflow trigger by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/117
* Check with is_in_mmtk_spaces instead of is_mapped_address by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/123
* Ask from binding if GC is disabled by @udesou in https://github.com/mmtk/mmtk-julia/pull/126
* Supporting moving immix by @udesou in https://github.com/mmtk/mmtk-julia/pull/93
* Stop using Julia's size classes when using MMTk by @udesou in https://github.com/mmtk/mmtk-julia/pull/108
* Hotfix alignment strings by @udesou in https://github.com/mmtk/mmtk-julia/pull/139
* Remove the coordinator thread by @wks in https://github.com/mmtk/mmtk-julia/pull/127
* Use to_address for SFT access by @wks in https://github.com/mmtk/mmtk-julia/pull/144
* Remove NULL ObjectReference by @wks in https://github.com/mmtk/mmtk-julia/pull/146
* Fix write barrier parameter type by @wks in https://github.com/mmtk/mmtk-julia/pull/148
* Update Julia to PR#48 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/149
* Rename edge to slot by @wks in https://github.com/mmtk/mmtk-julia/pull/150
* Update to MMTK core PR #1159 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/160
* Update Julia to latest master by @udesou in https://github.com/mmtk/mmtk-julia/pull/164
* Require ObjectReference to point inside object by @wks in https://github.com/mmtk/mmtk-julia/pull/173
* Update to mmtk-core PR #1205 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/177
* Updating our dev branches by @udesou in https://github.com/mmtk/mmtk-julia/pull/182
* Updating dev by @udesou in https://github.com/mmtk/mmtk-julia/pull/186
* Process pinning roots - port PR #142 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/192
* Adding `GC: MMTk` tag to Julia's banner when building with MMTk by @udesou in https://github.com/mmtk/mmtk-julia/pull/193
* Update to Julia PR #73 by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/194
* README.md: expand build instructions and put "quick start" first by @stephenrkell in https://github.com/mmtk/mmtk-julia/pull/195
* Deal with GC preserve stacks by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/191
* Move `bigval_t` struct to gc-common.h and loop through `GCAllocBytes` uses to apply fastpath allocation for MMTk by @udesou in https://github.com/mmtk/mmtk-julia/pull/196
* Support VO bit by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/197
* Updating dev to 2590e675 by @udesou in https://github.com/mmtk/mmtk-julia/pull/199
* Use MMTk's VO bit spec by @qinsoon in https://github.com/mmtk/mmtk-julia/pull/200
* Removing `WITH_MMTK` by @udesou in https://github.com/mmtk/mmtk-julia/pull/202
* Conservative stack scanning by @udesou in https://github.com/mmtk/mmtk-julia/pull/203

## New Contributors
* @kpamnany made their first contribution in https://github.com/mmtk/mmtk-julia/pull/29
* @NHDaly made their first contribution in https://github.com/mmtk/mmtk-julia/pull/47
* @k-sareen made their first contribution in https://github.com/mmtk/mmtk-julia/pull/63
* @wks made their first contribution in https://github.com/mmtk/mmtk-julia/pull/127
* @stephenrkell made their first contribution in https://github.com/mmtk/mmtk-julia/pull/195

**Full Changelog**: https://github.com/mmtk/mmtk-julia/commits/v0.29.0
