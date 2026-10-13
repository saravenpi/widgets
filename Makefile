BINS := statusbar sidebar launcher
PREFIX ?= $(HOME)/.local/bin

.PHONY: build install check clean

build:
	cargo build --release

check:
	cargo check --workspace

install: build
	@mkdir -p $(PREFIX)
	@cp target/release/$(BINS) $(PREFIX)/
	@echo "installed to $(PREFIX)"

clean:
	cargo clean