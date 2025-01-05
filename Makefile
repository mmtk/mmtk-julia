# By default do a release build with moving immix
MMTK_MOVING ?= 1
MMTK_PLAN ?= Immix
CURR_PATH := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Disable some variables set inside Julia 
# that may interfere with building the binding
PKG_CONFIG_LIBDIR=
PKG_CONFIG_PATH=

# If the Julia directory doesn't exist throw an error
# since we need it to generate the bindgen bindings
ifeq (${JULIA_PATH},)
$(error "JULIA_PATH must be set to generate Rust bindings")
endif

MMTK_JULIA_DIR := $(CURR_PATH)

PROJECT_DIRS := JULIA_PATH=$(JULIA_PATH) MMTK_JULIA_DIR=$(MMTK_JULIA_DIR)
MMTK_VARS := MMTK_PLAN=$(MMTK_PLAN) MMTK_MOVING=$(MMTK_MOVING)

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

# Build the mmtk-julia project
# Note that we might need to clone julia if it doesn't exist  
# since we need to run bindgen as part of building mmtk-julia
release:
	@echo "Building the Rust project in $(MMTK_JULIA_DIR)mmtk";
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

# Clean up the build artifacts
clean:
	@echo "Cleaning up build artifacts in $(JULIA_PATH) and $(MMTK_JULIA_DIR)";
	@cd $(MMTK_JULIA_DIR)mmtk && cargo clean

.PHONY: release debug julia julia-debug clean
