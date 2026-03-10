# Tracker MVP Implementation Plan

## Overview

A tracker-style music production app in Rust. Grid-based note entry (like OpenMPT), VST3 instrument hosting, effect routing, and mixing. MVP: note entry, playback, built-in sine wave instrument.

## Architecture

### Core Principle: Thin UI Shell

All application logic lives in a testable core. egui does two things: (1) translate input to `Action` values, (2) render `AppState` to screen.

```
┌─────────────────────────────────────┐
│       egui layer (thin shell)       │  input → Action, State → pixels
└──────────────┬──────────────────────┘
               │ Action enum
┌──────────────▼──────────────────────┐
│        App Core (testable)          │  AppState::apply(Action) → Vec<SideEffect>
└─────────────────────────────────────┘
```

### Thread Model

```
UI Thread (eframe)                    Audio Thread (cpal callback)
    │                                      │
    ├── rtrb::Producer<AudioCommand> ──►   rtrb::Consumer<AudioCommand>
    │                                      │
    │◄── Arc<AtomicUsize> (playback row) ──┤
    │                                      │
    │                                 SequencerState (owned)
    │                                 Vec<Box<dyn InstrumentPlugin>> (owned)
    │                                 Pattern (owned copy)
```

### Action/Reducer Pattern

Every user interaction becomes an `Action`. `AppState::apply()` processes it and returns `Vec<SideEffect>`. Side effects are data — tests assert on them without executing them.

```rust
enum Action {
    SetNote { row, channel, note },
    ClearNote { row, channel },
    MoveCursor(Direction),
    KeyPress(Key),
    Play, Stop,
    SetBpm(f64),
}

enum SideEffect {
    StartAudio,
    StopAudio,
    SendPatternToAudio(Pattern),
}
```

## Technology Stack

| Concern | Crate | Version |
|---------|-------|---------|
| GUI | eframe/egui | 0.33 |
| Audio output | cpal | 0.17 |
| Lock-free comms | rtrb | 0.3 |
| Serialization | serde + rmp-serde | 1.0 / 1.3 |

Future: `plugin_host` or `vst3-sys` for VST3 hosting.

## Data Model

```rust
struct Note {
    pitch: u8,       // MIDI 0-127, 0 = empty
    instrument: u8,
    velocity: u8,
}

struct Pattern {
    rows: usize,          // default 64
    channels: usize,      // default 4
    data: Vec<Vec<Note>>, // data[row][channel]
}
```

## Audio Engine

### Sine Wave Instrument

Phase-accumulator oscillator. `f = 440 * 2^((midi - 69) / 12)`.

```rust
trait InstrumentPlugin: Send {
    fn note_on(&mut self, pitch: u8, velocity: u8);
    fn note_off(&mut self, pitch: u8);
    fn render(&mut self, buffer: &mut [f32], sample_rate: f32);
}
```

8-voice polyphony, hard on/off (no envelope for MVP).

### Sequencer Timing

Runs inside the audio callback for sample-accurate timing.

```
samples_per_tick = sample_rate / ((bpm / 60.0) * rows_per_beat)
```

At BPM=120, rows_per_beat=4, sr=44100: ~5512.5 samples per row.

### render_block

Pure function — no hardware dependency. Takes `(output_buf, channels, sample_rate, sequencer_state, pattern, instruments)`. Advances sequencer, triggers notes at tick boundaries, renders instruments, sums to output.

## Module Structure

```
src/
  main.rs                -- eframe entry point
  app.rs                 -- TrackerApp: eframe::App impl
  core/
    mod.rs
    action.rs            -- Action, SideEffect, Direction enums
    state.rs             -- AppState + apply()
    pattern.rs           -- Note, Pattern
  audio/
    mod.rs
    engine.rs            -- AudioEngine (cpal + rtrb)
    render.rs            -- render_block(), SequencerState
    instrument.rs        -- InstrumentPlugin trait, TestSineInstrument
  ui/
    mod.rs
    pattern_editor.rs    -- Grid rendering via egui Painter
    toolbar.rs           -- Play/Stop/BPM
```

## Testing Strategy

~90% of code is testable without a UI or audio hardware.

### Test Categories

1. **Action/Reducer tests** (`core/state.rs`): Verify all state transitions.
   - Set/clear notes, cursor movement, play/stop side effects, keyboard-to-note mapping

2. **Audio render tests** (`audio/render.rs`): Verify audio correctness headlessly.
   - Silent when stopped, sequencer advances, notes produce output, timing accuracy

3. **Instrument tests** (`audio/instrument.rs`): Verify synthesis.
   - note_on produces output, note_off silences, frequency correctness via zero-crossing

4. **Serialization tests** (`core/pattern.rs`): Save/load roundtrip.

### Example Test

```rust
#[test]
fn type_a_note_into_cell() {
    let mut state = AppState::new();
    state.apply(Action::MoveCursor(Direction::Right)); // to channel 0
    state.apply(Action::KeyPress(Key::A)); // types note A

    let note = state.pattern.data[0][0];
    assert_eq!(note.pitch, 69 + 12); // A in current octave
    assert_eq!(state.cursor_row, 1); // cursor advanced
}
```

## Implementation Order

| Phase | Files | Testable? |
|-------|-------|-----------|
| 1. Data types | `core/pattern.rs`, `core/action.rs` | serde roundtrip |
| 2. App state | `core/state.rs` | action/reducer tests |
| 3. Sine instrument | `audio/instrument.rs` | synthesis tests |
| 4. Audio render | `audio/render.rs` | render_block tests |
| 5. Audio engine | `audio/engine.rs` | needs hardware |
| 6. UI grid | `ui/pattern_editor.rs` | visual only |
| 7. UI toolbar | `ui/toolbar.rs` | visual only |
| 8. App shell | `app.rs`, `main.rs` | run it |

## Future Work (Post-MVP)

- VST3 instrument hosting via `plugin_host` / `vst3-sys`
- VST3 effect plugins + effect chains
- Mixer panel (volume, pan, mute/solo per channel)
- Multiple patterns + song order
- Volume and effect columns in the grid
- MIDI keyboard input via `midir`
- Plugin GUI hosting (native windows)
- File save/load (.trk files via MessagePack)
- Undo/redo (action history)

## Key Risks

| Risk | Mitigation |
|------|------------|
| VST3 Rust hosting maturity | Start with built-in sine, add VST3 later. Fall back to C++ shim via FFI if needed. |
| egui grid performance | Custom Painter rendering, only draw visible rows. Pattern sizes are small (64-256 rows). |
| Audio glitches | render_block is allocation-free. Pattern sent as clone through ring buffer (~768 bytes). |
| Plugin GUI hosting | Defer to post-MVP. Most complex part of the project. |
