.PHONY: all
all:
	RUSTFLAGS='-C target-feature=+crt-static' cargo build -r --target x86_64-unknown-linux-gnu

.PHONY: .clean
clean:
	rm -rf ./target