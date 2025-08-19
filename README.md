# BLE Experiments

Minimal BLE Peripheral (Zephyr/C) + Central (Rust) to explore notifications end‑to‑end.

## Overview

- notifier (Zephyr, C): BLE Peripheral that advertises a 16‑bit service 0x183B with a single Notifiable characteristic (0x183C). It counts external events (button SW0 presses) and periodically notifies a 1‑byte value (sum of recent readings) every ~250 ms when a client subscribes.
- subscriber (Rust): BLE Central that scans for the service, connects, subscribes to the first characteristic in the service, and collects three notifications (with a 10 s timeout) using btleplug.

Service UUIDs (Bluetooth base UUID):
- Service: 0000183b-0000-1000-8000-00805f9b34fb
- Characteristic: 0000183c-0000-1000-8000-00805f9b34fb

## Repo structure

- notifier/: Zephyr application (C, CMake)
- subscriber/: Rust workspace member (btleplug + tokio)

## Prerequisites

- macOS, Linux, or Windows (tested logic is platform‑agnostic; RTT notes assume J‑Link targets on macOS/Linux)
- A Zephyr‑supported BLE board (e.g., Nordic nRF52 or nRF52840 DK). Board must provide DT alias `sw0` for the user button.
- Optional: SEGGER J‑Link for flashing/debug/RTT (RTT console is enabled in `prj.conf`).

---

## Zephyr setup (one‑time)

1) Install toolchain and West, reference the official guide (recommended): https://docs.zephyrproject.org/latest/develop/getting_started/index.html

```sh
cd embedded-workspace
python -m venv .venv
source .venv/bin/activate

pip install west

west init
west update

west zephyr-export
west packages pip --install

cd zephyr

west sdk install -t arm-zephyr-eabi
```

## Build and flash the notifier (Zephyr)

This repo (`ble-experiments`) contains a workspace app. You will build each embedded app from its own project directory.

From your project directory (e.g., `embedded-workspace`):

```sh
# Example for Nordic nRF52840 DK; change -b to your board (e.g., nrf52dk_nrf52832)
west build -p always -b nrf52840dk/nrf52840 
west flash --runner jlink
```

RTT logging (optional):

```sh
west debugserver
west rtt
```

Notes
- The app enables RTT and disables UART console (`prj.conf`). If your board lacks J‑Link/RTT, switch console to UART in `prj.conf`.
- The app uses DT alias `sw0` as the button input; ensure your board defines it (most Zephyr reference boards do).

---

## Rust setup (one‑time)

Install Rust and toolchain:

```sh
curl https://sh.rustup.rs -sSf | sh
rustup default stable
```

Platform specifics for BLE (btleplug):
- macOS: Works with CoreBluetooth. Grant Bluetooth permission to your terminal/IDE in System Settings > Privacy & Security > Bluetooth.
- Linux: Install BlueZ; ensure your user can access BLE (may need `bluetooth` group and running `bluetoothd`).
- Windows: Uses Windows BLE APIs; run from a terminal with Bluetooth enabled.

## Run the Rust subscriber

From the repo root:

```sh
cargo run -p subscriber
```

What it does
- Selects the first BLE adapter
- Scans for service 0000183b‑0000‑1000‑8000‑00805f9b34fb
- Connects, discovers services, subscribes to the first characteristic in that service
- Collects 3 notifications (times out after 10 s)

Expected behavior
- Press the board’s user button (SW0) a few times; the peripheral accumulates a 1‑byte sum and notifies periodically when subscribed.
- The subscriber logs three 1‑byte values and exits.

---

## End‑to‑end quick start

1) Flash the Zephyr notifier to your board and power it on.
2) Start RTT (optional) to see logs from the board.
3) Run `cargo run -p subscriber` on your desktop.
4) You should see a connection and three notifications within 10 seconds.

---

## Configuration reference

- Advertising name: `CONFIG_BT_DEVICE_NAME` in `notifier/prj.conf` (default: "Notifier 1")
- Advertising flags: General Discoverable, no BR/EDR
- Peripheral role, GATT dynamic DB, extended advertising enabled
- Notify period: ~250 ms (see `my_thread_entry_point` in `notifier/src/main.c`)
- Button: DT alias `sw0` (`notifier/src/external_readings.c`)
- Storage: simple heap‑backed list with a cap of 25 readings (`storage.c/.h`)

---

## Troubleshooting

- Subscriber can’t find adapters
	- Ensure Bluetooth is enabled; on macOS grant Bluetooth permission to your terminal/IDE; on Linux ensure BlueZ is running.
- No notifications
	- Ensure the client has subscribed; verify the service/characteristic UUIDs match; press SW0 to generate readings.
- No console output from the board
	- Use RTT (requires J‑Link) or switch to UART console by updating `prj.conf`.

---

## License

MIT (unless noted otherwise in subdirectories).

## Notes

This README was written by AI.