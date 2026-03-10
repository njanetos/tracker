# VST3 Integration Plan

## Overview

Add VST3 instrument (VSTi) hosting to the tracker. The architecture is already well-prepared: the `InstrumentPlugin` trait is the abstraction point — a VST3 instrument just needs to implement it.

## Key Challenges

1. **No mature Rust VST3 host crate exists.** The main option is `vst3-sys` (low-level FFI bindings to the Steinberg VST3 SDK COM interfaces). We build a safe wrapper on top.
2. **Instruments currently live on the audio thread** and are hardcoded as `TestSineInstrument` in `engine.rs`. We need a way to swap instruments per channel at runtime.
3. **Plugin scanning** — discovering `.vst3` bundles on disk (`~/Library/Audio/Plug-Ins/VST3/` on macOS).
4. **Plugin GUI hosting** — opening the plugin's native editor window. This is the hardest part and can be deferred.

## Phases

### Phase 1: Instrument Management Infrastructure (no VST3 yet)

Pure Rust, fully testable. Sets up the plumbing that Phase 2 plugs into.

#### Trait Changes (`src/audio/instrument.rs`)

Extend `InstrumentPlugin` with default methods:

```rust
pub trait InstrumentPlugin: Send {
    fn note_on(&mut self, pitch: u8, velocity: u8);
    fn note_off(&mut self, pitch: u8);
    fn render(&mut self, buffer: &mut [f32], sample_rate: f32);
    fn set_sample_rate(&mut self, _sample_rate: f32) {}  // default no-op
    fn name(&self) -> &str { "Unknown" }
}
```

#### Instrument Registry (`src/core/state.rs`)

- Add an instrument list to `AppState` tracking what's available and which instrument is assigned to each channel.
- A simple `InstrumentId` type (e.g., enum or string key) for identifying instrument types.
- For now, only `TestSineInstrument` is available. Adding VST3 later means registering new factories.

#### New Actions (`src/core/action.rs`)

```rust
Action::SetChannelInstrument { channel: usize, instrument_id: InstrumentId }
```

#### New SideEffects / AudioCommands

```rust
SideEffect::SetChannelInstrument { channel: usize, instrument_id: InstrumentId }
AudioCommand::SetInstrument { channel: usize, instrument: Box<dyn InstrumentPlugin> }
```

#### Engine Changes (`src/audio/engine.rs`)

- Handle `AudioCommand::SetInstrument` — swap out the instrument for a given channel in the audio callback.
- Ensure the old instrument gets proper note-off before replacement.

#### UI

- Per-channel instrument selector (dropdown in channel headers or sidebar).
- For now, only one option ("Sine"). Ready for more when VST3 lands.

#### Tests

- Action/reducer tests for `SetChannelInstrument`
- Verify side effects are emitted correctly
- Verify instrument swap triggers note-off on old instrument

### Phase 2: VST3 Plugin Scanning & Loading

- Add `vst3-sys` dependency to `Cargo.toml`
- New module: `src/audio/vst3/` with:
  - `host.rs` — `Vst3Host` struct implementing `IHostApplication`
  - `scanner.rs` — scan plugin directories, enumerate plugins
  - `plugin.rs` — `Vst3Instrument` implementing `InstrumentPlugin`
- Plugin discovery paths:
  - macOS: `~/Library/Audio/Plug-Ins/VST3/`, `/Library/Audio/Plug-Ins/VST3/`
  - Windows: `C:\Program Files\Common Files\VST3\`
  - Linux: `~/.vst3/`, `/usr/lib/vst3/`
- COM lifecycle: `IPluginFactory` -> `IComponent` -> `IAudioProcessor`
- Plugin metadata: name, vendor, category from `PClassInfo`

### Phase 3: Audio Processing Bridge

- Map `note_on`/`note_off`/`render` to VST3's `IAudioProcessor::process()`:
  - Build `ProcessData` with proper `Events` (MIDI note on/off as VST3 events)
  - Manage audio buffers (VST3 uses separate input/output buffer pointers)
  - Handle mono-to-stereo and buffer size differences
- Sample rate / block size negotiation via `IAudioProcessor::setupProcessing()`
- Plugin activation (`setActive(true)`) / deactivation lifecycle
- Handle plugins that need fixed block sizes (accumulate samples if needed)

### Phase 4: Plugin GUI (deferred)

- Host native plugin editor windows via `IPlugView`
- Platform-specific window handle management:
  - macOS: `NSView` parenting
  - Windows: `HWND` parenting
- Resize handling via `IPlugView::checkSizeConstraint()`
- Parameter changes from GUI -> host notification
- This is the most complex part and depends on the UI framework upgrade

## Design Notes

### Thread Safety

VST3 plugins are `Send` but not necessarily `Sync`. All plugin access happens on the audio thread (inside the cpal callback), which is correct. Plugin instantiation can happen on the UI thread, then the instance is sent to the audio thread via the ring buffer (`Box<dyn InstrumentPlugin>` is `Send`).

### The `instrument` Field in `Note`

The `Note` struct already has `instrument: u8`. Currently unused. Two options:
1. **Per-channel instrument** (current model): each channel has one instrument, the `instrument` field is ignored. Simple, matches classic tracker behavior.
2. **Per-note instrument**: the `instrument` field selects from a global instrument list, any channel can play any instrument. More flexible, matches IT/XM behavior.

Recommendation: start with per-channel (Phase 1), migrate to per-note later if desired.

### Instrument Factory Pattern

To create instruments on the UI thread and send them to the audio thread:

```rust
trait InstrumentFactory: Send + Sync {
    fn name(&self) -> &str;
    fn create(&self) -> Box<dyn InstrumentPlugin>;
}
```

Built-in instruments register factories at startup. VST3 scanner adds factories for each discovered plugin.
