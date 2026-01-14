# ezwhisper

Push-to-talk speech-to-text for macOS. Hold a button, speak, release - text appears at cursor.

## Features

- **Logitech MX Master support** - use the gesture button as trigger
- **Fast transcription** - ElevenLabs Scribe API (~200-500ms)
- **Optional AI cleanup** - Claude Haiku fixes grammar and punctuation
- **Menu bar status** - shows recording/transcribing state
- **Clipboard paste** - reliable text insertion via Cmd+V
- **Auto-Enter** - optionally send message after dictation

## Install

```bash
cargo install ezwhisper
```

Or build from source:
```bash
git clone https://github.com/piotrostr/ezwhisper
cd ezwhisper
cargo build --release
```

## Usage

```bash
# Basic usage
ELEVENLABS_API_KEY="your_key" ezwhisper

# With AI cleanup (fixes grammar/punctuation)
ELEVENLABS_API_KEY="..." ANTHROPIC_API_KEY="..." EZWHISPER_CLEANUP=true ezwhisper

# Auto-press Enter after paste (for chat apps)
ELEVENLABS_API_KEY="..." EZWHISPER_ENTER=true ezwhisper

# Select specific audio device
ELEVENLABS_API_KEY="..." EZWHISPER_DEVICE=0 ezwhisper
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ELEVENLABS_API_KEY` | Yes | ElevenLabs API key for transcription |
| `ANTHROPIC_API_KEY` | No | Anthropic API key for Haiku cleanup |
| `EZWHISPER_CLEANUP` | No | Enable AI text cleanup (default: false) |
| `EZWHISPER_ENTER` | No | Press Enter after paste (default: false) |
| `EZWHISPER_DEVICE` | No | Audio input device index |
| `EZWHISPER_LANGUAGE` | No | Language code (default: en) |

## Triggers

- **Logitech gesture button** - the large thumb button on MX Master mice
- **Right Option key** - works without any configuration

## Permissions Required

Grant these in System Settings > Privacy & Security:
- **Input Monitoring** - for detecting trigger button
- **Accessibility** - for simulating keyboard paste
- **Microphone** - for audio recording

## License

MIT
