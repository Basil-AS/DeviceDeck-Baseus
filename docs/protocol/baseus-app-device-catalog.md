# Baseus Intelligent app BLE device catalog

Extracted from the EU `app_device_ble_config` dictionary returned to Baseus Intelligent
2.18.0 (185) on 2026-11-08. The source contains 149 model-name rows in five groups.
`baseus-protocol::device_catalog::BASEUS_INTELLIGENT_218_EU` preserves each name and
its service/write/notify UUIDs.

## What this establishes

The application configures discovery for the listed names and supplies these GATT
profiles. It does **not** establish that every model in a shared UUID group uses the
same command set, that a BLE name matches exactly what a unit advertises, or that a
command has been accepted by hardware. The source is regional and may change server-side.

| Group | Entries | Profile / significance |
|---|---:|---|
| 1 | 26 | `EDFEC62E…` legacy accessory profile |
| 2 | 2 | X3/X5, standard `FFF0` service |
| 3 | 6 | Energy storage devices, `AE30` / `AE01` / `AE02` |
| 4 | 114 | Shared profile `53527AA4…` / `EE684B1A…` / `654B749C…` |
| 5 | 1 | BS-TS015, `1234` / `1235` / `1236` |

## Requested models

| Model | App catalog membership | Current command evidence |
|---|---|---|
| Blade 2 / Blade 2-140 | Group 4 | Blade 2-140 app-layer read/write codec is in `mobile_power`; hardware transport and model-specific capability confirmation remain open. |
| Bowie 30 | Group 4 | UUID profile only; command protocol not yet established. |
| Bowie MF1 | Group 4 | UUID profile only; app flow also contains classic RFCOMM for MF1; BLE command protocol not yet established. |
| Bass EP10 Pro | Group 4 | UUID profile and SPP app flow found; BLE command protocol and live transport not yet established. |

The model catalog intentionally keeps group 4 as `SharedProfileUnknown`. It must not be
used to send Blade 2 power-display commands to earbuds. Integrate each device after
advertising identity and first-frame/capability probing have been validated. Energy
storage devices in group 3 require a distinct protocol driver; their presence in the
catalog is not support for output-voltage/current telemetry.
