# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an ESP32-C6 embedded Rust project that implements a BLE GATT server for event monitoring. The system reads GPIO input events, tracks them over multiple time windows, and encodes the statistics into a UUID that's advertised via BLE. Devices can connect via GATT to receive event notifications.

## Architecture

### Workspace Structure

This is a Cargo workspace with two crates:

- **`ble-advertise`**: Main binary that runs on the ESP32-C6. Implements BLE advertising and GATT server functionality.
- **`event-storage`**: Library crate providing the event tracking system. Used by `ble-advertise` and includes tests.

### Hardware Target

- **Chip**: ESP32-C6 (RISC-V 32-bit)
- **Target**: `riscv32imac-unknown-none-elf` (configured in `.cargo/config.toml`)
- **Runner**: `probe-rs` for flashing via JTAG

### Runtime Environment

- **No standard library** (`no_std`) - uses `core` and `alloc` only
- **Executor**: `embassy-executor` for async task scheduling
- **RTOS**: `esp-rtos` provides Embassy integration with ESP hardware
- **Memory**: 64KB heap via `esp-alloc`
- **Logging**: `esp-println` with log level `Debug`

### Event Tracking System (`event-storage`)

The core data structure is `Events` (in `event-storage/src/storage.rs`), which:

- Stores up to 3000 events in a ring buffer (`HistoryBuf<u8, 3000>`)
- Tracks high/low events from GPIO input (1 for high, 0 for low)
- Maintains 8 time buckets: [1s, 5s, 30s, 60s, 120s, 240s, 360s, 600s]
- Calculates counts dynamically based on average duration between updates
- Encodes bucket counts into a UUID (8 buckets × 2 bytes = 16-byte UUID)

The system uses Embassy's `Mutex<NoopRawMutex>` for thread-safe access (single-core, no preemption needed).

### BLE Implementation (`ble-advertise`)

The main application (`ble-advertise/src/main.rs`) implements three concurrent tasks via `select3`:

1. **BLE host stack runner**: Manages BLE controller communication
2. **Event collector** (`collect_events`): Polls GPIO10 every 100ms and records high/low transitions
3. **Advertising/GATT loop** (`advertise`):
   - Advertises with UUID derived from event statistics
   - Accepts connections
   - Creates GATT attribute table with one characteristic (UUID 0x0001, notify property)
   - Waits for disconnection

BLE stack uses:
- `trouble-host` for host implementation
- `esp-radio` with `ExternalController` wrapper for ESP32 BLE radio
- `DefaultPacketPool` for packet management
- Connectable scannable undirected advertising

## Build and Development Commands

### Building

```bash
# Build for ESP32-C6 (target auto-configured)
cargo build

# Build specific crate
cargo build -p ble-advertise
cargo build -p event-storage

# Release build
cargo build --release
```

### Running/Flashing

```bash
# Flash to device and run (uses probe-rs)
cargo run

# Release run
cargo run --release
```

The runner automatically:
- Flashes via JTAG to ESP32-C6
- Pre-verifies binary
- Prints stack traces on panic
- Catches hard faults

### Testing

```bash
# Run tests for event-storage library
cargo test -p event-storage

# Run specific test
cargo test -p event-storage test_record_high_increments_all_buckets
```

Tests use `futures::executor::block_on` to test async code and simulate time using `Instant::from_ticks`.

## Important Constraints

### Embedded Environment

- `#![no_std]` - No standard library (must use `core`/`alloc`)
- `#![no_main]` - Entry point is `#[esp_rtos::main]`
- Stack size is limited
- Use `heapless` collections for fixed-size data structures
- Use `StaticCell` for static allocation of large types (like radio controller)
- All async code must work with Embassy's executor

### BLE Constraints

- Max advertising data: 32 bytes (in current implementation)
- Max scan response data: typically 31 bytes
- Data must be encoded using `AdStructure` from `bt-hci`
- UUID encoding: big-endian u16 values packed into 16-byte UUID

### Memory Management

- 64KB heap (configured in `main.rs:24`)
- Use static allocation where possible
- Ring buffers are fixed-size at compile time

## Key Dependencies

- `embassy-*`: Async runtime and HAL abstractions
- `esp-*`: ESP32 hardware support (HAL, allocator, bootloader, radio)
- `trouble-host`: BLE host stack implementation
- `bt-hci`: Bluetooth HCI types and encoding
- `heapless`: Fixed-capacity collections for `no_std`
- `static_cell`: Safe static mutable state initialization
