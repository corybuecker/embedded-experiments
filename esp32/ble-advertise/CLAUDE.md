# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an ESP32-C6 embedded Rust project for BLE (Bluetooth Low Energy) advertising. It uses the `no_std` embedded environment and runs on bare metal with the ESP32-C6 RISC-V microcontroller.

## Key Architecture

### Hardware Target

- **Chip**: ESP32-C6 (RISC-V 32-bit)
- **Target triple**: `riscv32imac-unknown-none-elf`
- **CPU Clock**: Configured to max speed

### Runtime Environment

- **Executor**: `embassy-executor` for async task management
- **RTOS**: `esp-rtos` with Embassy integration
- **Memory**: 128KB heap allocated via `esp-alloc`
- **Radio**: BLE controller via `esp-radio` and `trouble-host` stack

### Module Structure

**`src/main.rs`** - Application entry point

- Initializes hardware peripherals and Embassy runtime
- Sets up BLE controller and host stack
- Currently contains test code for event recording (advertising logic is commented out)

**`src/storage.rs`** - Time-based event tracking system

- `Events` struct tracks events across 8 time buckets (500ms to 5 minutes)
- Each `Bucket` maintains a count that decays over time
- Thread-safe using Embassy's `Mutex` (NoopRawMutex for single-core)
- Purpose: Track sensor readings or event frequency over different time windows

**`src/common.rs`** - Common imports and utilities

- Defmt RTT logging setup with microsecond timestamps
- Re-exports for backtrace and allocator

### BLE Stack

- Uses `trouble-host` for BLE host implementation
- `ExternalController` wraps the ESP32 BLE radio
- Supports non-connectable scannable advertising (code currently commented)
- Advertising data encoded with device name "Beacon1"

## Build and Development Commands

### Building

```bash
# Build for ESP32-C6 (automatically uses correct target from .cargo/config.toml)
cargo build

# Release build
cargo build --release
```

### Running/Flashing

```bash
# Build and flash to device (uses probe-rs runner configured in .cargo/config.toml)
cargo run

# Release run
cargo run --release
```

The runner command automatically:

- Flashes to ESP32-C6 chip
- Pre-verifies the binary
- Prints stack traces on panic
- Catches hard faults

### Logging

- Logging level controlled by `DEFMT_LOG` environment variable (set to "trace" in `.cargo/config.toml`)
- Uses `defmt` with RTT (Real-Time Transfer) for efficient embedded logging
- Timestamps show microseconds since boot

## Important Constraints

### Embedded Environment

- `#![no_std]` - No standard library (use `core` and `alloc` instead)
- `#![no_main]` - Custom entry point via `#[esp_rtos::main]`
- Fixed-size collections from `heapless` crate (no dynamic allocation for collections)
- All async code must be `Send` and work with Embassy's executor

### Memory Management

- Stack size is limited (typical embedded constraint)
- Heap size is 128KB
- Use `StaticCell` for static allocation of large types
- Prefer stack allocation or static storage when possible

### BLE Advertising Data

- Maximum advertising data: 64 bytes
- Maximum scan response data: 128 bytes
- Must be encoded using `AdStructure` types from `bt-hci`

## Development Notes

- The main advertising loop is currently commented out (lines 79-120 in `src/main.rs`)
- Test code records 3 high-priority events and reports bucket values after 5 seconds
- The `Events` system is designed to encode time-windowed event counts into BLE UUID service data
