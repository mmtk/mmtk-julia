# By default do a release build with moving immix
MMTK_MOVING ?= 1
MMTK_PLAN ?= Immix
CURR_PATH := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Disable some variables set inside Julia 
# that may interfere with building the binding
PKG_CONFIG_LIBDIR=
PKG_CONFIG_PATH=

MMTK_JULIA_DIR := $(CURR_PATH)

# If we need to generate the FFI bindings with bindgen
# and the Julia directory doesn't exist throw an error
ifeq ("$(wildcard $(MMTK_JULIA_DIR)/mmtk/src/julia_types.rs)","")
ifeq (${JULIA_PATH},) 
$(error "JULIA_PATH must be set to generate Rust bindings")
endif
endif

PROJECT_DIRS := JULIA_PATH=$(JULIA_PATH) MMTK_JULIA_DIR=$(MMTK_JULIA_DIR)
MMTK_VARS := MMTK_PLAN=$(MMTK_PLAN) MMTK_MOVING=$(MMTK_MOVING) MMTK_ALWAYS_MOVING=$(MMTK_ALWAYS_MOVING) MMTK_MAX_MOVING=$(MMTK_MAX_MOVING)

ifeq (${MMTK_PLAN},Immix)
CARGO_FEATURES = immix
else ifeq (${MMTK_PLAN},StickyImmix)
CARGO_FEATURES = stickyimmix
else
$(error "Unsupported MMTk plan: $(MMTK_PLAN)")
endif

ifeq ($(MMTK_MOVING), 0)
CARGO_FEATURES := $(CARGO_FEATURES),non_moving
endif

ifeq ($(MMTK_ALWAYS_MOVING), 1)
CARGO_FEATURES := $(CARGO_FEATURES),immix_always_moving
endif

ifeq ($(MMTK_MAX_MOVING), 1)
CARGO_FEATURES := $(CARGO_FEATURES),immix_max_moving
endif

# Build the mmtk-julia project
# Note that we might need to clone julia if it doesn't exist  
# since we need to run bindgen as part of building mmtk-julia
release:
	@echo "Building the Rust project in $(MMTK_JULIA_DIR)mmtk with MMTK_VARS: $(MMTK_VARS)";
	@cd $(MMTK_JULIA_DIR)mmtk && $(PROJECT_DIRS) cargo build --features $(CARGO_FEATURES) --release

debug:
	@echo "Building the Rust project in $(MMTK_JULIA_DIR) using a debug build";
	@cd $(MMTK_JULIA_DIR)mmtk && $(PROJECT_DIRS) cargo build --features $(CARGO_FEATURES) 

# Build the Julia project (which will build the binding as part of their deps build)
julia:
	@echo "Building the Julia project in $(JULIA_PATH)";
	@cd $(JULIA_PATH) && $(PROJECT_DIRS) $(MMTK_VARS) make

# Build the Julia project using a debug build (which will do a release build of the binding, unless MMTK_BUILD=debug)
julia-debug:
	@echo "Building the Julia project in $(JULIA_PATH)";
	@cd $(JULIA_PATH) && $(PROJECT_DIRS) $(MMTK_VARS) make debug

regen-bindgen-ffi:
	@echo "Regenerating the Rust FFI bindings for Julia (and rebuilding the binding)";
	@cd $(MMTK_JULIA_DIR) && rm -rf mmtk/src/julia_types.rs
	@$(MAKE) clean
	@$(MAKE) release

# Clean up the build artifacts
clean:
	@echo "Cleaning up build artifacts in $(JULIA_PATH) and $(MMTK_JULIA_DIR)";
	@cd $(MMTK_JULIA_DIR)mmtk && cargo clean

.PHONY: release debug julia julia-debug clean regen-bindgen-ffi
