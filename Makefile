# By default do a release build with moving immix
MMTK_MOVING ?= 1
MMTK_PLAN ?= Immix
CURR_PATH := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

include $(CURR_PATH)/julia.version

# If the Julia directory doesn't exist
# Clone it as a sibling of mmtk-julia
JULIA_PATH ?= $(CURR_PATH)../julia
MMTK_JULIA_DIR := $(CURR_PATH)

PROJECT_DIRS := JULIA_PATH=$(JULIA_PATH) MMTK_JULIA_DIR=$(MMTK_JULIA_DIR)
MMTK_VARS := MMTK_PLAN=$(MMTK_PLAN) MMTK_MOVING:=$(MMTK_MOVING)

ifeq (${MMTK_PLAN},Immix)
CARGO_FEATURES = immix
else ifeq (${MMTK_PLAN},StickyImmix)
CARGO_FEATURES = stickyimmix
else
$(error "Unsupported MMTk plan: $(MMTK_PLAN)")
endif

ifeq ($(MMTK_MOVING), 0)
CARGO_FEATURES += non_moving
endif

# Clone the repository if the directory does not exist
clone-julia:
	@if [ ! -d "$(JULIA_PATH)" ]; then \
		echo "Cloning repository from $(JULIA_GIT_URL)"; \
		git clone $(JULIA_GIT_URL) $(JULIA_PATH) --quiet; \
		cd $(JULIA_PATH) && \
		if git checkout $(JULIA_SHA1) --quiet; then \
			echo "Checked out commit $(JULIA_SHA1)"; \
		else \
			echo "Commit $(JULIA_SHA1) not found. Checking out tip of branch $(JULIA_BRANCH) instead."; \
			git checkout $(JULIA_BRANCH) --quiet; \
		fi; \
	else \
		echo "Directory $(JULIA_PATH) already exists. Skipping clone."; \
	fi

# Build the mmtk-julia project
# Note that we might need to clone julia if it doesn't exist  
# since we need to run bindgen as part of building mmtk-julia
binding: clone-julia
	@echo "Building the Rust project in $(MMTK_JULIA_DIR)mmtk";
	@cd $(MMTK_JULIA_DIR)mmtk && $(PROJECT_DIRS) cargo build --features $(CARGO_FEATURES) --release

binding-debug: clone-julia
	@echo "Building the Rust project in $(MMTK_JULIA_DIR) using a debug build";
	@cd $(MMTK_JULIA_DIR)mmtk && $(PROJECT_DIRS) cargo build --features $(CARGO_FEATURES) 

# Build the Julia project (which will build the binding as part of their deps build)
julia: clone-julia
	@echo "Building the Julia project in $(JULIA_PATH)";
	@cd $(JULIA_PATH) && $(MMTK_VARS) make

# Build the Julia project using a debug build (which will do a binding-debug as part of their deps build)
julia-debug: clone-julia
	@echo "Building the Julia project in $(JULIA_PATH)";
	@cd $(JULIA_PATH) && $(MMTK_VARS) make debug

# Clean up the build artifacts
clean:
	@echo "Cleaning up build artifacts in $(JULIA_PATH) and $(MMTK_JULIA_DIR)";
	@cd $(JULIA_PATH) && make clean
	@cd $(MMTK_JULIA_DIR)mmtk && cargo clean

.PHONY: clone-julia binding binding-debug julia julia-debug
