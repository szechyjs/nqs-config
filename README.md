# NQS Config

A desktop tool for reading and writing the configuration of the **NQS instrument
cluster ECU** (ECU 209 / 325) on the Ferrari F430, over CAN. Built with
[Tauri](https://tauri.app/) (Rust backend, React/TypeScript frontend).

It decodes and edits two bit-packed configuration blocks via KWP2000
(ISO 14230-3) `ReadDataByLocalIdentifier` / `WriteDataByLocalIdentifier`:

- **`$22`** — Engine variant (1× / 4× flywheel)
- **`$24`** — Vehicle options: units, gearbox type, nation, clock format,
  TPMS, oil temp sensor, body style, driver side, fuel restriction, buzzers,
  brake type, oil pressure sensor

See `src-tauri/src/nqs.rs` for the full bit layout and
`src-tauri/src/kwp.rs` for the KWP2000 service/security-access details.

## Installation

Prebuilt binaries for Windows and macOS are available on the
[Releases page](https://github.com/szechyjs/nqs-config/releases).

To build from source instead, see [Development](#development) below.

## Hardware

Communication runs over a [CANable](https://canable.io/) USB-to-CAN adapter
using the SLCAN protocol at 50 kbps, wired to the vehicle's EOBD connector:

| Signal | OBD-II Pin |
| ------ | ---------- |
| CAN H  | 1          |
| GND    | 5          |
| CAN L  | 9          |

Disconnect the CANable's onboard 120 Ω termination resistor jumper before
connecting — the vehicle's bus is already terminated. (This wiring diagram is
also shown in-app via the **Help** button.)

## Development

Prerequisites: [Rust](https://www.rust-lang.org/), [Node.js](https://nodejs.org/),
[pnpm](https://pnpm.io/), the [Tauri CLI](https://v2.tauri.app/reference/cli/)
(`cargo install tauri-cli`), and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
pnpm install
cargo tauri dev    # run in development
cargo tauri build  # produce a release bundle
```

Rust unit tests (bit-packing/decoding logic):

```sh
cd src-tauri && cargo test
```

## Disclaimer

This tool writes directly to ECU EEPROM over a diagnostic session. Incorrect
or inconsistent configuration values may cause unexpected cluster/ECU
behavior. Use at your own risk; always verify changes after writing.

## License

[GNU AGPL-3.0-or-later](./LICENSE). You're free to use, modify, and
redistribute this software, including commercially — but if you distribute a
modified version (or run it as a network service), you must make that
version's source available under the same license. This also means the
software itself can't be repackaged and resold as a closed-source product.

Copyright (C) 2026 JSS Technologies, LLC
