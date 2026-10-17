BINS := statusbar sidebar launcher exit settings
PREFIX ?= $(HOME)/.local/bin

.PHONY: build install check clean

build:
	cargo build --release

check:
	cargo check --workspace
	filet check -fail warn

install: build
	@mkdir -p $(PREFIX)
	@cp $(addprefix target/release/,$(BINS)) $(PREFIX)/
	@echo "installed to $(PREFIX)"

clean:
	cargo clean