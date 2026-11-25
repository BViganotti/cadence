# Cadence development commands.
#
# No CI is used. These targets are conveniences for a human or an agent.

CARGO ?= cargo

.PHONY: all build test test-release fmt-check check bench fuzz fixtures studio clean

all: build test

build:
	$(CARGO) build --workspace --release

test:
	$(CARGO) test --workspace

test-release:
	$(CARGO) test --workspace --release

check:
	$(CARGO) build --workspace
	$(CARGO) test --workspace

bench:
	$(CARGO) run --release -p cadence-bench

fuzz:
	$(CARGO) run --release -p cadence-fuzz

fixtures:
	$(CARGO) run --release -p cadence-fixtures

studio:
	$(CARGO) run --release -p cadence

clean:
	$(CARGO) clean
