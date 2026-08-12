# OpenCarpanel SCS telemetry bridge

This native SCS Telemetry SDK plugin sends a fixed 44-byte versioned datagram to
`127.0.0.1:20777` after each unpaused game frame. It supports Euro Truck
Simulator 2 and American Truck Simulator.

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
`opencarpanel-scs-telemetry.dll` on Windows,
`opencarpanel-scs-telemetry.dylib` on macOS, and
`opencarpanel-scs-telemetry.so` on Linux.

## Install

- Windows: copy the DLL into `<game>/bin/win_x64/plugins/`.
- macOS: copy the dylib into
  `<game>.app/Contents/MacOS/plugins/`.
- Linux: copy the SO into `<game>/bin/linux_x64/plugins/`.

Create the `plugins` directory if it does not exist. On the next game launch,
accept the game's notice that an SDK plugin is active. OpenCarpanel Host must
listen on its default telemetry port, UDP 20777.

The plugin has a distinct filename and can coexist with ETS2LA's
`scs-telemetry` shared-memory plugin.

## Wire protocol v1

All integers and IEEE-754 floats are little-endian. The exact layout is:

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 4 | Magic `OCP\0` |
| 4 | 1 | Protocol version (`1`) |
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

The corresponding defensive decoder lives in `crates/adapter-scs`.

## Licensing

OpenCarpanel bridge code is Apache-2.0. The vendored SCS SDK headers are under
SCS Software's permissive SDK license; that notice is included in installed
packages as `sdk_license.txt`.
