PHONY: all build clean fmt fmt-check init lint pre-commit test full-test

all: init build full-test

clean:
	@echo ──────────── Clean ────────────────────────────
	@rm -rvf target

build:
	@echo ⚙️ Building a release...
	@cargo +nightly b -r --workspace
	@ls -l target/wasm32-unknown-unknown/release/*.wasm

fmt:
	@echo ⚙️ Formatting...
	@cargo fmt --all

fmt-check:
	@echo ⚙️ Checking a format...
	@cargo fmt --all --check

init:
	@echo ⚙️ Installing a toolchain \& a target...
	@rustup toolchain add nightly
	@rustup target add wasm32-unknown-unknown --toolchain nightly

lint:
	@echo ⚙️ Running the linter...
	@cargo +nightly clippy -- -D warnings
	@cargo +nightly clippy \
	--all-targets \
	--workspace \
	-Fbinary-vendor \
	-- -D warnings

pre-commit: fmt lint full-test

deps:
	@echo ⚙️ Downloading dependencies...
	@if [ ! -f "./target/fungible_token-0.1.3.wasm" ]; then\
		wget "https://github.com/gear-dapps/fungible-token/releases/download/0.1.3/fungible_token-0.1.3.wasm"\
			-O "./target/fungible_token-0.1.3.wasm";\
	fi

test: deps
	@echo ⚙️ Running unit tests...
	@cargo +nightly t

node-test: deps
	wget https://get.gear.rs/gear-nightly-linu\x-x86_64.tar.xz && \
	tar xvf gear-nightly-linux-x86_64.tar.xz && \
	rm gear-nightly-linux-x86_64.tar.xz
	@./gear --dev --tmp > /dev/null 2>&1  & echo "$$!" > gear.pid
	cat gear.pid;
	@cargo test -Fbinary-vendor --package staking --test node_test -- --include-ignored --test-threads=1; 	kill `(cat gear.pid)`; rm gear; rm gear.pid

full-test: deps
	@echo ⚙️ Running all tests...
	@cargo +nightly t -Fbinary-vendor -- --include-ignored --test-threads=1
