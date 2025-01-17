## An MMTk binding for the Julia programming language.

### Quick Building Guide

To quickly build Julia with MMTk, check out Julia from its main repository instead and create a `Make.user` file containing `MMTK_PLAN=Immix`. 

```
git clone https://github.com/JuliaLang/julia
cd julia && echo 'MMTK_PLAN=Immix' > Make.user && make
```

This should automatically check out a (binary) version of this repo, and link it when building Julia itself.

To build only the mmtk-julia binding from source, run:

```
git clone https://github.com/mmtk/mmtk-julia -b master
(cd mmtk-julia && make release)  # or "make debug" for a debug build
```

If you would like debugging information in your release build of MMTk, add `debug = true` under `[profile.release]` in `mmtk/Cargo.toml`.
Below, we provide some instructions on how to build the mmtk-julia binding from source _and_ link it when building Julia.

### Checking out and Building Julia with MMTk

If you'd like to try out Julia with MMTk, simply clone the Julia repository from https://github.com/JuliaLang/julia and create a `Make.user` file inside the `julia` folder containing `MMTK_PLAN=Immix`. This will automatically checkout the latest release of the mmtk-julia binding and link it while building Julia itself.

To build the binding from source, besides checking out this repository, it is also necessary to checkout a version of the Julia repository (https://github.com/JuliaLang/julia). We recommend checking out the latest master, but any commit after [this](https://github.com/JuliaLang/julia/commit/22134ca28e92df321bdd08502ddd86ad2d6d614f) should work.
For example, we check out Julia as a sibling of `mmtk-julia`.

The directory structure should look like the diagram below:

```
Your working directory/
├─ mmtk-julia/
│  └─ mmtk/
├─ julia/ (should be cloned manually)
└─ mmtk-core/ (optional)
```

#### Build Julia binding in Rust

Before building Julia, build the binding in `mmtk-julia`. Set `MMTK_JULIA_DIR` to the absolute path containing the binding's top-level directory and build the binding by running `make release` or `make debug` from that directory.

We currently only support a (non-moving) Immix implementation. We hope to add support for non-moving StickyImmix and the respective moving versions of both collectors in the near future. We also only support x86_64 Linux, more architectures should also be supported in the near future.
For a release build with debugging information, first add `debug = true` under `[profile.release]` in `mmtk/Cargo.toml`.
Make sure you have the prerequisites for building [MMTk](https://github.com/mmtk/mmtk-core#requirements).

#### Build Julia with MMTk (from source)

To build Julia with MMTk using the version built in the previous step, first ensure you have the prerequisites for building [Julia](https://github.com/JuliaLang/julia/blob/master/doc/src/devdocs/build/build.md#required-build-tools-and-external-libraries).

Next create a `Make.user` file in the top-level directory of the Julia repository consisting of the line `MMTK_PLAN=Immix`.

Finally, if you have not done it already, set the following environment variable:

```
export MMTK_JULIA_DIR=<path-to-mmtk-julia>
```
... and run `make` from Julia's top-level directory.

Alternatively you can set the environment variables in your `Make.user` 

```
export MMTK_JULIA_DIR := <path-to-mmtk-julia>
export MMTK_PLAN := Immix
```

If you have done a debug build of the binding, make sure to also set `MMTK_BUILD=debug` before building Julia.

### Rust FFI bindings from Julia

The mmtk-julia binding requires a set of Rust FFI bindings that are automatically generated from the Julia repository using [bindgen](https://github.com/rust-lang/rust-bindgen). In this repository, the FFI bindings have already been generated, and added to the file `mmtk/src/julia_types.rs`. 
However, if Julia changes the object representation of any of the types defined in the FFI bindings in `mmtk/src/julia_types.rs`, that file will become outdated.
To generate the FFI bindings again (and rebuild the binding), checkout the Julia repository following the steps described [previously](#checking-out-and-building-julia-with-mmtk), set the environment variable `JULIA_PATH` to point to the `julia` directory and run `make regen-bindgen-ffi` from the binding's top-level directory, note that this step will already do a release build of mmtk-julia containing the new version of `julia_types.rs`. Make sure you have all the [requirements](https://rust-lang.github.io/rust-bindgen/requirements.html) to running `bindgen`.

### Heap Size

Currently MMTk supports a fixed heap limit or variable heap within an interval. The default is a variable heap with the minimum heap size set to Julia's [`default_collection_interval`](https://github.com/mmtk/julia/blob/847cddeb7b9ddb5d6b66bec4c19d3a711748a45b/src/gc.c#L651) and the maximum size set to 70% of the free memory available. To change these values set the environment variables `MMTK_MIN_HSIZE` and `MMTK_MAX_HSIZE` to set the mininum and maximum size in megabytes, or `MMTK_MIN_HSIZE_G` and `MMTK_MAX_HSIZE_G` to set the size in gigabytes. If both environment variables are set, MMTk will use the size in megabytes. To set a fixed heap size, simply set only the variables `MMTK_MAX_HSIZE` or `MMTK_MAX_HSIZE_G`, or set `MMTK_MIN_HSIZE` or `MMTK_MIN_HSIZE_G` to 0. Note that these values can be decimal numbers, e.g. `MMTK_MAX_HSIZE_G=1.5`.

These environment variables are set during julia initialization time, so they can be set per-julia process.
 
### Further information

More about MMTk: https://github.com/mmtk/mmtk-core

More about Julia: https://github.com/JuliaLang/julia
