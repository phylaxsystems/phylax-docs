# Heavily inspired by reth : https://github.com/paradigmxyz/reth/blob/4de1ff069124240c6d4b0c7d3d5d5556b74a1eae/Makefile

.DEFAULT_GOAL := help

GIT_TAG ?= $(shell git describe --tags --abbrev=0)
BIN_DIR = "dist/bin"

BUILD_PATH = "target"

# Cargo profile for builds. Default is for local builds, CI uses an override.
PROFILE ?= release

# Extra flags for Cargo
CARGO_INSTALL_EXTRA_FLAGS ?=

# The docker image name
DOCKER_IMAGE_NAME ?= ghcr.io/phylax-systems/phylax

# Features
FEATURES = := min-error-logs min-warn-logs min-info-logs min-debug-logs min-trace-logs

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Build

.PHONY: install
install: ## Build and install the phylax binary under `~/.cargo/bin`.
	cargo install --path bin/phylax --bin phylax --force --locked \
		--features "$(FEATURES)" \
		--profile "$(PROFILE)" \
		$(CARGO_INSTALL_EXTRA_FLAGS)

# Builds the phylax binary natively.
build-native-%:
	cargo build --bin phylax --target $* --features "$(FEATURES)" --profile "$(PROFILE)"

# The following commands use `cross` to build a cross-compile.
#
# These commands require that:
#
# - `cross` is installed (`cargo install cross`).
# - Docker is running.
# - The current user is in the `docker` group.
#
# The resulting binaries will be created in the `target/` directory.

zigbuild-%: 
	cargo zigbuild --bin phylax --target $* --features "$(FEATURES)" --profile "$(PROFILE)"

build-%: 
	cross build --bin phylax --target $* --features "$(FEATURES)" --profile "$(PROFILE)"

build-x86_64-apple-darwin:
	$(MAKE) build-native-x86_64-apple-darwin
build-aarch64-apple-darwin:
	$(MAKE) build-native-aarch64-apple-darwin

# Create a `.tar.gz` containing a binary for a specific target.
define tarball_release_binary
	cp $(BUILD_PATH)/$(1)/$(PROFILE)/$(2) $(BIN_DIR)/$(2)
	cd $(BIN_DIR) && \
		tar -czf phylax-$(GIT_TAG)-$(1)$(3).tar.gz $(2) && \
		rm $(2)
endef

# The current git tag will be used as the version in the output file names. You
# will likely need to use `git tag` and create a semver tag (e.g., `v0.2.3`).
#
# Note: This excludes macOS tarballs because of SDK licensing issues.
.PHONY: build-release-tarballs
build-release-tarballs: ## Create a series of `.tar.gz` files in the BIN_DIR directory, each containing a `phylax` binary for a different target.
	[ -d $(BIN_DIR) ] || mkdir -p $(BIN_DIR)
	$(MAKE) build-x86_64-unknown-linux-gnu
	$(call tarball_release_binary,"x86_64-unknown-linux-gnu","phylax","")
	$(MAKE) build-aarch64-unknown-linux-gnu
	$(call tarball_release_binary,"aarch64-unknown-linux-gnu","phylax","")

##@ Test

UNIT_TEST_ARGS := --locked --workspace --all-features -E 'kind(lib)' -E 'kind(bin)' -E 'kind(proc-macro)'
COV_FILE := lcov.info

.PHONY: test-unit
test-unit: ## Run unit tests.
	cargo nextest run $(UNIT_TEST_ARGS)

.PHONY: cov-unit
cov-unit: ## Run unit tests with coverage.
	rm -f $(COV_FILE)
	cargo llvm-cov nextest --lcov --output-path $(COV_FILE) $(UNIT_TEST_ARGS)

.PHONY: cov-report-html
cov-report-html: cov-unit ## Generate a HTML coverage report and open it in the browser.
	cargo llvm-cov report --html
	open target/llvm-cov/html/index.html

##@ Docker

# Note: This requires a buildx builder with emulation support. For example:
#
# `docker run --privileged --rm tonistiigi/binfmt --install amd64,arm64`
# `docker buildx create --use --driver docker-container --name cross-builder`
.PHONY: docker-build-latest
docker-build-latest: ## Build and push a cross-arch Docker image tagged with the latest git tag and `latest`.
	$(call build_docker_image,$(GIT_TAG),latest)

# Note: This requires a buildx builder with emulation support. For example:
#
# `docker run --privileged --rm tonistiigi/binfmt --install amd64,arm64`
# `docker buildx create --use --name cross-builder`
.PHONY: docker-build-nightly
docker-build-nightly: ## Build and push cross-arch Docker image tagged with the latest git tag with a `-nightly` suffix, and `latest-nightly`.
	$(call build_docker_image,$(GIT_TAG)-nightly,latest-nightly)

# Create a cross-arch Docker image with the given tags and push it
define build_docker_image
	$(MAKE) build-x86_64-unknown-linux-gnu
	mkdir -p $(BIN_DIR)/amd64
	cp $(BUILD_PATH)/x86_64-unknown-linux-gnu/$(PROFILE)/phylax $(BIN_DIR)/amd64/phylax

	$(MAKE) build-aarch64-unknown-linux-gnu
	mkdir -p $(BIN_DIR)/arm64
	cp $(BUILD_PATH)/aarch64-unknown-linux-gnu/$(PROFILE)/phylax $(BIN_DIR)/arm64/phylax

	docker buildx build --file ./Dockerfile.cross . \
		--platform linux/amd64,linux/arm64 \
		--tag $(DOCKER_IMAGE_NAME):$(1) \
		--tag $(DOCKER_IMAGE_NAME):$(2) \
		--provenance=false \
		--push
endef

##@ Other

.PHONY: clean
clean: ## Perform a `cargo` clean and remove the binary and test vectors directories.
	cargo clean
	rm -rf $(BIN_DIR)
