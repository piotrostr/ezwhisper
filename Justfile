# run the app
start:
    cargo run --release

# development with live reload
dev:
    cargo watch -x 'run --release'

# build release binary
build:
    cargo build --release

# run tests
test:
    cargo test
