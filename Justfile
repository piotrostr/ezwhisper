# run the app
start:
    cargo run --release --bin ezwhisper

# run with Polish to English translation
start-pl-en:
    EZWHISPER_TRANSLATE=true cargo run --release --bin ezwhisper

# development with live reload
dev:
    EZWHISPER_ENTER=true cargo watch -x 'run --release --bin ezwhisper'

# development with Polish to English translation
dev-pl-en:
    EZWHISPER_ENTER=true EZWHISPER_TRANSLATE=true cargo watch -x 'run --release --bin ezwhisper'

# build release binary
build:
    cargo build --release

# run tests
test:
    cargo test
