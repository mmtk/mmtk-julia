## An MMTk binding for the Julia programming language.

### Checking out and Building Julia with MMTk

Besides checking out the binding, it is also necessary to checkout a fork containing a modified version of the Julia repository (https://github.com/mmtk/julia).
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

Before building Julia, build the binding in `mmtk-julia/mmtk`. Note that we currently support either immix or stickyimmix implementations in mmtk-core (build it with `cargo build --features immix` or `cargo build --features stickyimmix`). Add `--release` at the end if you would like to have a release build, otherwise it is a debug build.

#### Build Julia with MMTk

To build Julia with MMTk, create a `Make.user` file in the top-level directory of the Julia repository and add an entry `WITH_MMTK=1`. Finally, set the following environment variables:

```
export MMTK_BUILD=release # or debug depending on how you build the Julia binding in Rust
export MMTK_JULIA_DIR=<path-to-mmtk-julia>
```

Then run `make` with the environment variables mentioned above. Please also make sure to install any dependency considering any particular requirement from both [Julia](https://github.com/JuliaLang/julia/blob/master/doc/src/devdocs/build/build.md#required-build-tools-and-external-libraries) and [MMTk](https://github.com/mmtk/mmtk-core#requirements). 

### Heap Size

Currently MMTk supports a fixed heap limit or variable heap within an interval. The default is a variable heap with the minimum heap size set to Julia's [`default_collection_interval`](https://github.com/mmtk/julia/blob/847cddeb7b9ddb5d6b66bec4c19d3a711748a45b/src/gc.c#L651) and the maximum size set to 70% of the free memory available. To change these values set the environment variables `MMTK_MIN_HSIZE` and `MMTK_MAX_HSIZE` to set the mininum and maximum size in megabytes, or `MMTK_MIN_HSIZE_G` and `MMTK_MAX_HSIZE_G` to set the size in gigabytes. If both environment variables are set, MMTk will use the size in megabytes. To set a fixed heap size, simply set only the variables `MMTK_MAX_HSIZE` or `MMTK_MAX_HSIZE_G`, or set `MMTK_MIN_HSIZE` or `MMTK_MIN_HSIZE_G` to 0. Note that these values can be decimal numbers, e.g. `MMTK_MAX_HSIZE_G=1.5`.

These environment variables are set during julia initialization time, so they can be set per-julia process.
 
### Quick Building Guide

(1) Clone this repo: https://github.com/mmtk/mmtk-julia (run `git clone https://github.com/mmtk/mmtk-julia`)

(2) Clone this repo: https://github.com/mmtk/julia (run `git clone https://github.com/mmtk/julia.git`)

(3) In `mmtk-julia/mmtk`, run `cargo build --features immix --release`

(4) In `julia`, create a file `Make.user`, and add `WITH_MMTK=1`.

(5) In `julia`, run `MMTK_PLAN=Immix MMTK_BUILD=release MMTK_JULIA_DIR=../mmtk-julia make` (or with `MMTK_PLAN=StickyImmix`).

If you would like to have a debug build, remove `--release` from Step (3) and use `MMTK_BUILD=debug` in Step (5)

### Further information

More about MMTk: https://github.com/mmtk/mmtk-core

More about Julia: https://github.com/JuliaLang/julia
