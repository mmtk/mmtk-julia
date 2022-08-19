## An MMTk binding for the Julia programming language.

### Checking out and Building Julia with MMTk

Besides checking out the binding, it is also necessary to checkout a fork containing a modified version of the Julia repository on to a subfolder of the biding (eg. `vm/julia`). The correct version can be found at https://github.com/mmtk/julia/tree/mmtk-julia-master. Note that the hashcode for the correct version should also match the version specified in the `Cargo.toml` file under the `mmtk` folder. For step-by-step instructions, read the section "Quick Building Guide". 

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
export MMTK_JULIA_DIR=<path-to-mmtk-julia>/mmtk
```

Before building Julia, build the Rust binding in `mmtk-julia/mmtk`. Note that we currently support the Immix implementation in mmtk-core (build it with `cargo build --features immix`). To build Julia, navigate to `/vm/julia` and run `make` (or `make debug`). Please also make sure to install any dependency considering any particular requirement from both [Julia](https://github.com/JuliaLang/julia/blob/master/doc/src/devdocs/build/build.md#required-build-tools-and-external-libraries) and [MMTk](https://github.com/mmtk/mmtk-core#requirements). 
To build MMTk, we use the Rust version specified in the [CI script](https://github.com/udesou/mmtk-julia/blob/34014217512f97d3e524350af8ab2beb997fdb3f/.github/scripts/ci-setup.sh#L15).

### Heap Size

Currently MMTk only supports a fixed heap size. The default size set when initializing the GC is 6GB. It is possible to change this size dinamically, by setting the 
environment variables `MMTK_HEAP_SIZE` to set the size in MB or `MMTK_HEAP_SIZE_G` to set the size in GB. Either value can be a decimal number, e.g. `MMTK_HEAP_SIZE_G=1.5` and if both variables are set, `MMTK_HEAP_SIZE` is accounted for.
 
### Quick Building Guide

(1) Clone this repo: https://github.com/udesou/mmtk-julia/tree/mmtk-julia-master 
  (run `git clone https://github.com/udesou/mmtk-julia.git -b mmtk-julia-master`)

(2) Enter the mmtk-julia directory and clone https://github.com/udesou/julia/tree/mmtk-julia-master 
  (run `git clone https://github.com/udesou/julia.git vm/julia -b mmtk-julia-master`)

(3) inside `mmtk-julia/mmtk` run `cargo build --features immix # --release (optional - needs to match the MMTK_BUILD variable)` and
inside `mmtk-julia/vm/julia` run `make # debug (optional)`, making sure you have the environment variables above set up and a file in `mmtk-julia/vm/julia` named "Make.user" containing `USE_MMTK=1`.

### Further information

More about MMTk: https://github.com/mmtk/mmtk-core

More about Julia: https://github.com/JuliaLang/julia
