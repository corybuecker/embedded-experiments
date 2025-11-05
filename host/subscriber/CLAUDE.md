# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a BLE Central (host) application written in Rust that scans for and connects to BLE peripherals advertising a specific service UUID. It's part of a larger embedded-experiments monorepo that includes various BLE peripheral implementations (Zephyr/C, Embassy/Rust, ESP32) and this subscriber client for testing them.

The subscriber:
1. Scans for BLE devices advertising service UUID `0000183b-0000-1000-8000-00805f9b34fb`
2. Connects to the first matching peripheral
3. Discovers the characteristic `0000183c-0000-1000-8000-00805f9b34fb`
4. Subscribes to notifications
5. Collects 3 notifications (with a 10-second timeout)
6. Logs the received 1-byte values

## Repository Context

This subscriber is located at `host/subscriber/` within a monorepo at `/Users/corybuecker/projects/embedded-experiments/`. The repo contains:

- **`embedded-workspace/`**: Zephyr BLE peripherals (C/CMake) - beacon, notify, periodic
- **`nRF52840/`**: Embassy/Rust BLE peripherals for nRF52840
- **`esp32/`**: ESP32-C6 BLE GATT server (see `esp32/event-beacon/CLAUDE.md`)
- **`pico-2w/`**: Raspberry Pi Pico W experiments
- **`host/`**: This workspace (subscriber + read-usb)
- **`probe-rs-api/`**: Probe-rs API experiments

The subscriber works with any of the BLE peripheral implementations that advertise the expected service/characteristic UUIDs.

## Workspace Structure

This is part of a Cargo workspace defined in `host/Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["read-usb", "subscriber"]
```

The workspace contains:
- **`subscriber`** (this project): BLE Central client using btleplug
- **`read-usb`**: USB device reading utility

## Architecture

### Module Structure

- **`src/main.rs`**: Entry point, initializes logging and BLE adapter, calls scanner functions
- **`src/scanner.rs`**: Core BLE logic for scanning, connecting, and collecting notifications

### Key Components

**main.rs:13-64** - Sets up tracing subscriber for logging and initiates the BLE workflow:
1. Initializes btleplug Manager
2. Gets first available BLE adapter
3. Calls `connect()` to find and connect to peripheral
4. Calls `wait_for_notify()` to collect notifications

**scanner.rs:18-26** - `connect()` function wraps connection logic with a 5-second timeout

**scanner.rs:28-64** - `connect_to_peripheral()` implements the connection flow:
- Scans for devices with service UUID filter
- Waits 1280ms (BLUETOOTH_ADVERTISING_INTERVAL) for advertising packets
- Connects to first matching peripheral
- Discovers services and characteristics
- Subscribes to the first characteristic in the service

**scanner.rs:66-74** - `wait_for_notify()` wraps notification collection with a 10-second timeout

**scanner.rs:76-105** - `collect_samples()` collects exactly 3 notifications:
- Takes 3 values from notification stream
- Validates each is 1 byte
- Decodes as little-endian u8 values
- Returns count of collected samples

### BLE Configuration

- **Service UUID**: `0000183b-0000-1000-8000-00805f9b34fb` (defined in scanner.rs:15)
- **Characteristic UUID**: `0000183c-0000-1000-8000-00805f9b34fb` (first characteristic in service)
- **Scan timeout**: 1280ms (to match peripheral advertising interval)
- **Connection timeout**: 5 seconds
- **Notification timeout**: 10 seconds
- **Sample count**: 3 notifications

## Build and Development Commands

### Building

```bash
# Build from workspace root (host/)
cd /Users/corybuecker/projects/embedded-experiments/host
cargo build -p subscriber

# Build from this directory
cargo build

# Release build
cargo build --release
```

### Running

```bash
# Run from workspace root
cargo run -p subscriber

# Run from this directory
cargo run
```

### Linting

```bash
# Run clippy (must pass with no warnings for CI)
cargo clippy -- -D warnings
```

The CI workflow (`.github/workflows/branch-protection.yaml`) requires clippy to pass with `-D warnings` on Linux with `libdbus-1-dev` and `pkg-config` installed.

## Platform-Specific Requirements

### macOS
- Uses CoreBluetooth (built-in)
- Grant Bluetooth permission to terminal/IDE in System Settings > Privacy & Security > Bluetooth
- No additional dependencies needed

### Linux
- Requires BlueZ and DBus libraries
- Install: `sudo apt-get install -y libdbus-1-dev pkg-config`
- User may need to be in `bluetooth` group
- Ensure `bluetoothd` daemon is running

### Windows
- Uses Windows BLE APIs
- Ensure Bluetooth is enabled in system settings

## Key Dependencies

- **`btleplug` (0.11)**: Cross-platform BLE library
  - Uses CoreBluetooth on macOS
  - Uses BlueZ/DBus on Linux
  - Uses Windows BLE APIs on Windows
- **`tokio` (1.46.1)**: Async runtime with features: `rt-multi-thread`, `macros`, `time`, `process`
- **`tokio-stream` (0.1.17)**: Stream utilities for collecting notifications
- **`tracing` + `tracing-subscriber`**: Structured logging
- **`anyhow` (1.0.98)**: Error handling with context
- **`uuid` (1.17.0)**: UUID parsing for service/characteristic UUIDs

## Common Patterns

### Async Timeout Pattern
The code uses a consistent pattern for timeout handling:
```rust
let join_handle = spawn(async_operation(args));
match timeout(Duration::from_millis(N), join_handle).await {
    Ok(Ok(result)) => Ok(result?),
    Ok(Err(e)) => Err(anyhow!("Error message: {e}")),
    Err(_) => Err(anyhow!("Timeout message")),
}
```

### Error Handling
- Uses `anyhow::Result` for ergonomic error propagation
- Provides context with `anyhow!()` macro
- Logs errors using `tracing::error!`

### Notification Collection
Uses `tokio_stream::StreamExt` to:
1. Take exactly N notifications (`.take(3)`)
2. Map to extract value bytes
3. Validate data format
4. Collect into Vec

## Testing with Peripherals

To test this subscriber, you need a BLE peripheral advertising the expected service. Options in this repo:

1. **Zephyr (nRF52840 DK)**:
   ```bash
   cd embedded-workspace/peripheral-notify
   west build -p always -b nrf52840dk/nrf52840
   west flash --runner jlink
   ```

2. **Embassy/Rust (nRF52840)**:
   ```bash
   cd nRF52840/ble-notification
   cargo build --release
   probe-rs download target/thumbv7em-none-eabihf/release/ble-notification --chip nRF52840_xxAA
   ```

3. **ESP32-C6**:
   ```bash
   cd esp32/event-beacon/ble-advertise
   cargo run --release
   ```

Then press the button (SW0) on the peripheral board to generate events that will be notified to the subscriber.

## Commit Message Format

This repository uses conventional commits. From `.github/prompts/git-commit.prompt.md`:
- Keep subject line under 72 characters
- Use conventional commit format (e.g., `feat:`, `fix:`, `refactor:`, `test:`, `docs:`)
- Write detailed body explaining the "why" rather than "what"
- Do not stage all files at once; group logically related changes
