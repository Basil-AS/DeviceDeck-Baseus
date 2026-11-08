# Baseus Blade 2-140 mobile-power protocol (static extraction)

**Source:** user-provided Baseus Intelligent 2.18.0 (version code 185), arm64 bundle. The Android view-model builds these application payloads before passing them to Baseus's BLE SDK. This note and the Rust codec are derived from APK code; they are not a capture from a Blade 2 and do not establish hardware support.

## Frame construction

The app hex-decodes a textual payload, calculates the CRC in `EnergyStorageUtil`, appends the four-digit CRC text, then hands the resulting bytes to its BLE manager. The app-layer formats are:

```text
read:       9A AA 03 CODE_H CODE_L 00 01 CRC_H CRC_L
write u16:  9A AA 10 CODE_H CODE_L 00 01 02 01 VALUE_H VALUE_L CRC_H CRC_L
write text: 9A AA 10 00 30 00 01 BYTE_LEN 01 UTF8_BYTES... CRC_H CRC_L
```

`crates/baseus-protocol/src/mobile_power.rs` implements those builders and a CRC-checked common status decoder. Its CRC routine follows the table and loop in the APK's `EnergyStorageUtil.m31711f`. The vendor app's BLE SDK adds/manages the actual GATT connection; this codec does not know the model's dynamically supplied service and characteristic UUIDs.

## Features visible in app code

| Feature | Code(s) | App behavior |
|---|---|---|
| Main dashboard | `000D`, `0014`, `0010`, `0015`, `0016`, `0011`, `001E`, `001F`, `0013`, `0023` | Queries temperature, energy/electricity, status, remaining/charge time, error, reminder/shutdown timers and module version. Blade 2-140 additionally requests `0025`, `0026`, `0024` for cell temperature, cycles and DC-module version. |
| Screen theme | `0020` | Three preset indices 0–2; not an arbitrary RGB color picker. |
| Screen-saver state | `0022` | Read/toggle enabled state. |
| Custom text | `0030` | UTF-8 text; app sends byte length and a `01` selector. APK filters input and ignores empty text. The hardware maximum and complete character repertoire are unknown. |
| Reminder / auto-off | `001E`, `001F` | Read and write countdown values; app clamps displayed values at 1440 minutes. |
| Type-C measurements | `0001`–`000C`, `0012` | Two ports show input/output voltage, current, power and protocol. Statistic values use `raw / response_multiplier`. |
| Cell statistics | `0025`, `0026`, `0028`–`002D`, `0031`–`0036` | Temperature, cycle count, six cell voltages and six cell currents; `0010` also feeds six displayed cell-status bits at bit positions 6–11. |
| Port toggles | `0018`–`001B` | App writes 0/1 based on the state parsed from code `0010`. Exact mapping of each code to the physical port needs a live trace. |
| Statistics session | `0037`, `0038` | App enables/polls these while the statistics view is active and sends 0 on exit. Semantics are not named by the static code. |
| Defaults reset | `000E` | A reset write exists in the app. Not exposed by this codec and should require an explicit user-confirmation flow if ever added. |

Protocol labels for `0012` include PPS, PD 3.0, QC 2.0/3.0, AFC, FCP and SCP, with a model-specific index table. The app's labels are not a guarantee that every port/accessory negotiates every standard.

## Validation status and next capture

- Static builder/parser paths and code-to-UI mappings are confirmed in the APK.
- The value multiplier, physical units, CRC behavior on a real unit, BLE UUIDs, notification framing and success/readback behavior are **not** hardware-verified.
- Capture a Blade 2-140 session with the official app: save GATT service/characteristics, one read and response per code, then separately change theme, screen text, screen-saver, each timer and each port state. Record firmware and verify the displayed values against a USB-C meter before enabling live writes.
- Do not add model registration, discovery matching or writes that affect charging/battery safety until the transport and command mapping are verified.

## Code provenance

The Rust codec is deliberately a pure protocol layer. This separation makes the app-derived frame shape independently testable without implying that the UI or BLE transport supports Blade 2 yet.
