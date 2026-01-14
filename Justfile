# run the app
start:
    cargo run --release --bin ezwhisper

# development with live reload
dev:
    EZWHISPER_ENTER=true cargo watch -x 'run --release --bin ezwhisper'

# build release binary
build:
    cargo build --release

# run tests
test:
    cargo test
