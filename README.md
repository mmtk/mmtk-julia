## An MMTk binding for the Julia programming language.

### Quick Building Guide

```
git clone https://github.com/mmtk/mmtk-julia
git clone https://github.com/mmtk/julia
(cd julia && git checkout dev && echo 'MMTK_PLAN=Immix' > Make.user) # or MMTK_PLAN=StickyImmix to use Sticky Immix
export JULIA_PATH=`pwd`/julia
export MMTK_JULIA_DIR=`pwd`/mmtk-julia
(cd mmtk-julia/mmtk && cargo build --features immix --release)   # or drop "--release" for a debug build
MMTK_BUILD=release MMTK_JULIA_DIR=`pwd`/mmtk-julia make -C julia  # or "MMTK_BUILD=debug"
```

If you would like debugging information in your release build of MMTk, add `debug = true` under `[profile.release]` in `mmtk/Cargo.toml`.

### Checking out and Building Julia with MMTk

Besides checking out the binding (this repository), it is also necessary to checkout a fork containing a modified version of the Julia repository (https://github.com/mmtk/julia).
_Use the `dev` branch of the Julia repository._
For example, we check out the fork as a sibling of `mmtk-julia`.
For step-by-step instructions, read the section "Quick Building Guide".

The directory structure should look like the diagram below:

```
Your working directory/
├─ mmtk-julia/
│  ├─ julia/
│  └─ mmtk/
├─ julia/ (should be cloned manually)
└─ mmtk-core/ (optional)
```

#### Build Julia binding in Rust

Before building Julia, build the binding in `mmtk-julia/mmtk`. You must have already checked out Julia, set `JULIA_PATH` to point to the checkout (remember to check out the `dev` branch!) and set `MMTK_JULIA_DIR` to the binding's top-level directory.

In `mmtk-core` we currently support either Immix or StickyImmix implementations.
Build it with `cargo build --features immix` or `cargo build --features stickyimmix`.
Add `--release` at the end if you would like to have a release build, otherwise it is a debug build.
For a release build with debugging information, first add `debug = true` under `[profile.release]` in `mmtk/Cargo.toml`.

#### Build Julia with MMTk

To build Julia with MMTk, first ensure you have the prerequisites for building both [Julia](https://github.com/JuliaLang/julia/blob/master/doc/src/devdocs/build/build.md#required-build-tools-and-external-libraries) and [MMTk](https://github.com/mmtk/mmtk-core#requirements).

Next create a `Make.user` file in the top-level directory of the Julia repository consisting of the line `MMTK_PLAN=Immix` or `MMTK_PLAN=StickyImmix`.

Finally, set the following environment variables:

```
export MMTK_BUILD=release # or debug depending on how you build the Julia binding in Rust
export MMTK_JULIA_DIR=<path-to-mmtk-julia>
```
... and run `make`.

Alternatively you can set the environment variables in your `Make.user` 

```
export MMTK_BUILD := release
export MMTK_JULIA_DIR := <path-to-mmtk-julia>
export MMTK_PLAN := Immix # or export MMTK_PLAN := StickyImmix
```

### Heap Size

Currently MMTk supports a fixed heap limit or variable heap within an interval. The default is a variable heap with the minimum heap size set to Julia's [`default_collection_interval`](https://github.com/mmtk/julia/blob/847cddeb7b9ddb5d6b66bec4c19d3a711748a45b/src/gc.c#L651) and the maximum size set to 70% of the free memory available. To change these values set the environment variables `MMTK_MIN_HSIZE` and `MMTK_MAX_HSIZE` to set the mininum and maximum size in megabytes, or `MMTK_MIN_HSIZE_G` and `MMTK_MAX_HSIZE_G` to set the size in gigabytes. If both environment variables are set, MMTk will use the size in megabytes. To set a fixed heap size, simply set only the variables `MMTK_MAX_HSIZE` or `MMTK_MAX_HSIZE_G`, or set `MMTK_MIN_HSIZE` or `MMTK_MIN_HSIZE_G` to 0. Note that these values can be decimal numbers, e.g. `MMTK_MAX_HSIZE_G=1.5`.

These environment variables are set during julia initialization time, so they can be set per-julia process.
 
### Further information

More about MMTk: https://github.com/mmtk/mmtk-core

More about Julia: https://github.com/JuliaLang/julia
