# Nocturne Optimizer

Windows-first optimizer built with Tauri 2, Rust, React and TypeScript.

## 01. Live Optimization / Selected Profile

This is the main workflow.

- `Selected Profile` is now the primary control surface.
- Processes are grouped into real app families instead of raw flat process spam.
- One rule can cover the full family:
  - Discord + updater + helper chain
  - Chrome + Google Update + renderer helpers
  - Edge + WebView / updater helpers
- HUD toggle is handled by Rust, so it still reacts when the frontend is under load.

## Preview Slots

| Live Optimization | Overview | HUD |
| --- | --- | --- |
| ![Live Optimization Placeholder](docs/screenshots/live-selected-profile-placeholder.svg) | ![Overview Placeholder](docs/screenshots/overview-shadow-telemetry-placeholder.svg) | ![HUD Placeholder](docs/screenshots/hud-overlay-placeholder.svg) |


## Core Modules

### 01. Overview
<img width="2547" height="1699" alt="image" src="https://github.com/user-attachments/assets/c071a3e7-8e34-4008-9ebe-c89a1bae0bb5" />

- rebuilt telemetry hero
- grouped heavy app families
- lower refresh pressure on the UI
- selected profile summary surfaced in the first screen

### 02. Live Optimization
<img width="2560" height="1796" alt="image" src="https://github.com/user-attachments/assets/03863122-6d45-442b-a33c-03172e6e67f3" />

- dark family picker instead of unreadable white selects
- grouped app family table with helper breakdown
- sticky selected profile panel
- per-family CPU / RAM / Disk / GPU sliders

### 03. Autostart
<img width="2036" height="987" alt="image" src="https://github.com/user-attachments/assets/710c8e30-dbe8-4871-8685-311d8cac58b2" />

- scans:
  - `HKCU/HKLM Run`
  - `RunOnce`
  - `Policies\\Explorer\\Run`
  - `RunServices`
  - `WOW6432Node` startup keys
  - Startup folders
  - Scheduled Tasks
  - Services
- loading state while startup sources are being collected

### 04. Offline Optimization
<img width="2073" height="1422" alt="image" src="https://github.com/user-attachments/assets/6ea0a516-369b-457b-b687-130a5496e339" />

- temp cleaner
- background quiet preset
- debloat-lite preset
- loading state for Windows apps and installed program inventory

### 05. Registry Health
<img width="2034" height="967" alt="image" src="https://github.com/user-attachments/assets/03d028ae-48f2-4fb0-a335-7ea534624753" />

- critical security and platform checks
- scan console
- repair console

### 06. Security
<img width="2077" height="1229" alt="image" src="https://github.com/user-attachments/assets/1e0fa55b-5f7e-4a6d-84f6-3ac234c1f24a" />

- app protection password
- relock on restore / activate
- startup password for Nocturne

### 07. Network
<img width="2049" height="870" alt="image" src="https://github.com/user-attachments/assets/83b1a694-6c3b-406f-9541-67b60bd37af4" />

- adapter overview
- stored per-process network rule plans
- loading state while adapters and rules are fetched

### 08. Settings / HUD
<img width="1490" height="1737" alt="image" src="https://github.com/user-attachments/assets/b9136856-f6c7-4629-b815-4c82ccd2f6b0" />
<img width="631" height="352" alt="image" src="https://github.com/user-attachments/assets/b250a48c-bc56-4518-95a6-4db49ea6e02b" />

- global translucent HUD
- Rust-driven show / hide hotkey
- manual HUD toggle button
- HUD placement designer

## Stack

- Backend: Rust
- Shell: Tauri 2
- Frontend: React + TypeScript + Vite
- UI: custom CSS + Lucide icons

## Development

```bash
npm install
npm run tauri dev
```

## Production Build

```bash
npm install
npm run build
npm run tauri build
```

Installer output:

```text
src-tauri\target\release\bundle\nsis\
```

## Notes

- Project is Windows-first.
- Some actions require Administrator privileges.
- Guard overlay does not replace Windows logon or Secure Desktop.
- Network rules are stored as process-oriented profiles. Full driver-level bandwidth enforcement would require deeper WFP integration.

## Reset Local State

Delete stale config files between experimental builds:

```text
%LOCALAPPDATA%\NocturneOptimizer\security.json
%LOCALAPPDATA%\NocturneOptimizer\settings.json
%LOCALAPPDATA%\NocturneOptimizer\rules.json
%LOCALAPPDATA%\NocturneOptimizer\network-rules.json
```
