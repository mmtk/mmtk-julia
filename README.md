## An MMTk binding for the Julia programming language.

### Checking out and Building Julia with MMTk

Besides checking out the binding, it is also necessary to checkout a fork containing a modified version of the Julia repository onto a subfolder of the biding (eg. `vm/julia`). The correct version can be found at https://github.com/mmtk/julia/tree/master. For step-by-step instructions, read the section "Quick Building Guide". 

The directory structure should look like the diagram below:

```
Your working directory/
├─ mmtk-julia/
│  ├─ julia/
│  ├─ mmtk/
│  └─ vm/julia # should be cloned manually
└─ mmtk-core/ (optional)
```

To build Julia with MMTk, create a `Make.user` file in the top-level directory of the Julia repository (`vm/julia/`) and add an entry `USE_MMTK=1`. Finally, set the following environment variables:

```
export MMTK_BUILD=release # or debug depending on how you build the Rust binding
export MMTK_JULIA_DIR=<path-to-mmtk-julia>
```

Before building Julia, build the Rust binding in `mmtk-julia/mmtk`. Note that we currently support either immix or marksweep implementations in mmtk-core (build it with `cargo build --features immix # or marksweep`). To build Julia, navigate to `/vm/julia` and run `make` (or `make debug`). Please also make sure to install any dependency considering any particular requirement from both [Julia](https://github.com/JuliaLang/julia/blob/master/doc/src/devdocs/build/build.md#required-build-tools-and-external-libraries) and [MMTk](https://github.com/mmtk/mmtk-core#requirements). 

### Heap Size

Currently MMTk supports a fixed heap limit or variable heap within an interval. The default is a variable heap with the minimum heap size set to Julia's [`default_collection_interval`](https://github.com/mmtk/julia/blob/847cddeb7b9ddb5d6b66bec4c19d3a711748a45b/src/gc.c#L651) and the maximum size set to 70% of the free memory available. To change these values set the environment variables `MMTK_MIN_HSIZE` and `MMTK_MAX_HSIZE` to set the mininum and maximum size in megabytes, or `MMTK_MIN_HSIZE_G` and `MMTK_MAX_HSIZE_G` to set the size in gigabytes. If both environment variables are set, MMTk will use the size in megabytes. To set a fixed heap size, simply set only the variables `MMTK_MAX_HSIZE` or `MMTK_MAX_HSIZE_G`, or set `MMTK_MIN_HSIZE` or `MMTK_MIN_HSIZE_G` to 0. Note that these values can be decimal numbers, e.g. `MMTK_MAX_HSIZE_G=1.5`.

These environment variables are set during julia initialization time, so they can be set per-julia process.
 
### Quick Building Guide

(1) Clone this repo: https://github.com/mmtk/mmtk-julia 
  (run `git clone https://github.com/mmtk/mmtk-julia -b v1.8.2-RAI`)

(2) Enter the mmtk-julia directory and clone https://github.com/mmtk/julia/tree/v1.8.2-RAI 
  (run `git clone https://github.com/mmtk/julia.git vm/julia -b v1.8.2-RAI`)

(3) inside `mmtk-julia/mmtk` run `cargo build --features immix # --release (optional - needs to match the MMTK_BUILD variable)` (run `cargo build --features marksweep` to use marksweep instead) and
inside `mmtk-julia/vm/julia` run `make # debug (optional)`, making sure you have the environment variables above set up and a file in `mmtk-julia/vm/julia` named "Make.user" containing `USE_MMTK=1`.

### Further information

More about MMTk: https://github.com/mmtk/mmtk-core

More about Julia: https://github.com/JuliaLang/julia
