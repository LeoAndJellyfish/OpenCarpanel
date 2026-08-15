# OpenSimDash SCS telemetry bridge

This native SCS Telemetry SDK plugin sends a fixed 188-byte v2 datagram to
`127.0.0.1:20777` after each unpaused game frame. It supports Euro Truck
Simulator 2 and American Truck Simulator. The Rust Host also accepts legacy
44-byte v1 packets from older plugin builds.

The frame callback performs no dynamic allocation, locking, DNS lookup, file
I/O, or blocking network operation. It sends only to the IPv4 loopback address.

## Build

```sh
cmake -S . -B build -DBUILD_TESTING=ON
cmake --build build --config Release
ctest --test-dir build -C Release --output-on-failure
cmake --install build --config Release --prefix package
```

Use a 64-bit compiler. The resulting file is named
`opensimdash-scs-telemetry.dll` on Windows,
`opensimdash-scs-telemetry.dylib` on macOS, and
`opensimdash-scs-telemetry.so` on Linux.

## Install

- Windows: copy the DLL into `<game>/bin/win_x64/plugins/`.
- macOS: copy the dylib into
  `<game>.app/Contents/MacOS/plugins/`.
- Linux: copy the SO into `<game>/bin/linux_x64/plugins/`.

Create the `plugins` directory if it does not exist. On the next game launch,
accept the game's notice that an SDK plugin is active. OpenSimDash Host must
listen on its default telemetry port, UDP 20777.

The plugin has a distinct filename and can coexist with ETS2LA's
`scs-telemetry` shared-memory plugin.

## Wire protocol v2

All integers and IEEE-754 floats are little-endian. Bytes 0–43 retain the v1
base layout; the exact v2 layout is:

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 4 | Magic `OSD\0` |
| 4 | 1 | Protocol version (`2`) |
| 5 | 1 | Game (`1` ETS2, `2` ATS) |
| 6 | 2 | Flags and reserved bytes (`0`) |
| 8 | 8 | Session nonce |
| 16 | 4 | Frame sequence |
| 20 | 4 | Signed speed, m/s |
| 24 | 4 | Engine RPM |
| 28 | 4 | RPM limit, or zero while unknown |
| 32 | 4 | Displayed gear |
| 36 | 4 | Effective throttle, 0–1 |
| 40 | 4 | Effective brake, 0–1 |
| 44 | 4 | Navigation distance, m |
| 48 | 4 | Navigation time, s |
| 52 | 4 | Navigation speed limit, m/s |
| 56 | 4 | Fuel amount, L |
| 60 | 4 | Fuel capacity, L |
| 64 | 4 | Fuel range, km |
| 68 | 2 | Parking/headlight/beacon/brake/reverse/indicator/hazard bits |
| 70 | 2 | Fuel warning/job active/cargo loaded/special-job bits |
| 72 | 4 | Delivery time |
| 76 | 4 | Planned distance, simulated km |
| 80 | 8 | Job income |
| 88 | 4 | Cargo mass, kg |
| 92 | 32 | Cargo name, UTF-8 with zero padding |
| 124 | 32 | Source city, UTF-8 with zero padding |
| 156 | 32 | Destination city, UTF-8 with zero padding |

The corresponding defensive decoder lives in `crates/adapter-scs`. It selects
v1 or v2 from the version byte, requires the exact associated length, and
rejects unknown bits, invalid text padding/UTF-8, and invalid numeric values.

## Licensing

OpenSimDash bridge code is Apache-2.0. The vendored SCS SDK headers are under
SCS Software's permissive SDK license; that notice is included in installed
packages as `sdk_license.txt`.
