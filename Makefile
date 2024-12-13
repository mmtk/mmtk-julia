# By default do a release build with moving immix
MMTK_MOVING ?= 1
MMTK_PLAN ?= Immix
CURR_PATH := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Getting metadata about Julia repo from Cargo file
JULIA_GIT_URL= $(shell cargo read-manifest --manifest-path=$(CURR_PATH)/mmtk/Cargo.toml | python -c 'import json,sys; print(json.load(sys.stdin)["metadata"]["julia"]["julia_repo"])')
JULIA_VERSION= $(shell cargo read-manifest --manifest-path=$(CURR_PATH)/mmtk/Cargo.toml | python -c 'import json,sys; print(json.load(sys.stdin)["metadata"]["julia"]["julia_version"])')

# If the Julia directory doesn't exist
# Clone it as a sibling of mmtk-julia
JULIA_PATH ?= $(CURR_PATH)../julia
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

# Clone the repository if the directory does not exist
clone-julia:
	@if [ ! -d "$(JULIA_PATH)" ]; then \
		echo "Cloning repository from $(JULIA_GIT_URL)"; \
		git clone $(JULIA_GIT_URL) $(JULIA_PATH) --quiet; \
		cd $(JULIA_PATH) && \
		if git checkout $(JULIA_VERSION) --quiet; then \
			echo "Checked out commit $(JULIA_VERSION)"; \
		else \
			echo "Error: Commit $(GIT_COMMIT) does not exist."; \
			exit 1; \
		fi; \
	else \
		echo "Directory $(JULIA_PATH) already exists. Skipping clone-julia."; \
	fi

# Build the mmtk-julia project
# Note that we might need to clone julia if it doesn't exist  
# since we need to run bindgen as part of building mmtk-julia
release: clone-julia
	@echo "Building the Rust project in $(MMTK_JULIA_DIR)mmtk";
	@cd $(MMTK_JULIA_DIR)mmtk && $(PROJECT_DIRS) cargo build --features $(CARGO_FEATURES) --release

debug: clone-julia
	@echo "Building the Rust project in $(MMTK_JULIA_DIR) using a debug build";
	@cd $(MMTK_JULIA_DIR)mmtk && $(PROJECT_DIRS) cargo build --features $(CARGO_FEATURES) 

# Build the Julia project (which will build the binding as part of their deps build)
julia: clone-julia
	@echo "Building the Julia project in $(JULIA_PATH)";
	@cd $(JULIA_PATH) && $(PROJECT_DIRS) $(MMTK_VARS) make

# Build the Julia project using a debug build (which will do a release build of the binding, unless MMTK_BUILD=debug)
julia-debug: clone-julia
	@echo "Building the Julia project in $(JULIA_PATH)";
	@cd $(JULIA_PATH) && $(PROJECT_DIRS) $(MMTK_VARS) make debug

# Clean up the build artifacts
clean:
	@echo "Cleaning up build artifacts in $(JULIA_PATH) and $(MMTK_JULIA_DIR)";
	@cd $(JULIA_PATH) && make clean
	@cd $(MMTK_JULIA_DIR)mmtk && cargo clean

.PHONY: clone-julia release debug julia julia-debug clean
