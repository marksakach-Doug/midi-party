# MIDI Party

A midi workstation with good keybindings.

## Build and run

Requires a Rust toolchain (rustc 1.95+). From the repo root:

```bash
cargo run -p mp-app --release
```

Optional: a `.ron` project or `.mid` file to open, and `--sf2 <file>` for the
default SoundFont. A tiny public-domain test bank ships in `assets/soundfonts/`;
for real instruments download a General MIDI bank such as **GeneralUser GS**
(32 MB) or **MuseScore_General** (MIT) and pass it with `--sf2`, or set it per
track with `:sf2 path/to/bank.sf2 <preset>`.

## Keys

Press `Shift+/` for the live cheat sheet; `Cmd+k` opens the keymap editor. The basics:

| Keys | Action |
|---|---|
| arrows / Shift+arrows / Ctrl+Left,Right | cursor by grid step / bar & octave / beat |
| `n`, `Shift+n` | insert a note / a major chord at the cursor (`4 n` = four grid steps long) |
| `x`, `Delete` | delete the selection (or the note under the cursor) |
| `s`, `v`, `Shift+v` | toggle note under cursor / box select / time-range select; `Enter` confirms |
| `a`, `b`, `w`, `Shift+s`, `Shift+p`, `i` | select all / bar / next 4 notes / chord / same pitch / invert |
| `Tab`, `Shift+Tab`, `Shift+x` | step through the selection, drop the focused note |
| `f t` `f b` `f e` `f o` `f a` `f l` | keep only: top / bottom of chords, every other, on-beat, above / below cursor |
| `f i` `f r` `f x` `f u` `f d`; or `Alt+i/r/x/u/d` | melodic inversion / retrograde / retrograde inversion / augmentation / diminution |
| `m` then arrows | move the selected notes; `[` `]` length, `-` `=` velocity, `Alt+Left/Right` nudge off-grid |
| `c` `p` `d` `.` | copy, paste at cursor, duplicate, repeat last edit |
| `q`, `Shift+q`, `h`, `Shift+l` | quantize starts / lengths, humanize, legato |
| `u` / `Shift+u` | undo / redo |
| `g` / `Shift+g` | finer / coarser grid |
| `t` / `Shift+t`, `Cmd+t`, `Cmd+Shift+t` | next / previous track, add track, remove track |
| `Ctrl+m`, `Ctrl+s`, `Ctrl+=`/`-`, `Ctrl+[`/`]` | mute, solo, track volume, track pan |
| `Ctrl+.` / `Ctrl+,` | next / previous SoundFont preset |
| `Shift+c`, `Ctrl+Shift+Up/Down`, `Ctrl+Shift+d/u` | cycle controller lane, nudge lane value, sustain pedal down / up |
| `Space`, `Enter` | play/stop, play from cursor |
| `:` | command line: `:w song.ron`, `:o file.mid`, `:export out.mp3`, `:bpm 96`, `:meter 3/4`, `:vel 70..110`, `:quantize 1/8 50%`, `:rename Bass`, `:sf2 bank.sf2 33`, `:lane ramp 0..127`, `:q` |

Type a number before any key to repeat it. The mouse works too: click to place
the cursor or select, Shift+click to toggle, double-click to insert, drag to
move, drag a note's right edge to resize, drag empty space to box-select,
wheel to scroll, pinch or Cmd+wheel to zoom.

On Linux, connect a USB MIDI keyboard from the input selector in the bottom bar,
then click **Connect**. Incoming notes play through the selected track's
SoundFont. **MIDI softest** sets a velocity floor (default 40), lifting quiet
keypresses into an audible range while preserving the louder end. **Record** starts from the current transport position (or the editor
cursor) and writes note timing and velocity into the piano roll; pitch bend,
sustain and modulation are recorded to their controller lanes. Recorded MIDI
edits share an undo step. The **Metronome** checkbox is enabled by default,
accenting each bar's first beat during playback and recording. If the keyboard
does not appear, enable the ALSA sequencer and refresh the input list.

For untimed entry, use **Step record**: each MIDI key press inserts a note at
the cursor, gives it the current grid length, and advances by one grid step.
Change the step size with `g` / `Shift+g` before or during entry. Step recording
does not require matching the metronome.

## Configuration

On first run the app writes `keymap.toml` and `theme.toml` to its config folder:
`~/Library/Application Support/midi-party` on macOS, `~/.config/midi-party` on Linux.
Both are *overlays* on the built-in defaults: anything in them overrides, the
rest keeps tracking updates. `preset = "vim"` at the top of `keymap.toml`
switches cursor movement to `h j k l`. Set a key to `"unbind"` to remove it.
Both files reload the moment you save them. `Cmd+e` hides and shows the piano
roll under the tracks area, and `Cmd+0` fits the whole song in the tracks area again.

The built-in look is the warm dark palette in `themes/default.toml`; copy
`themes/party.toml` over your `theme.toml` for the louder one. `track_colors`
lists the note colors by track number.

## macOS app

```bash
scripts/bundle-macos.sh
```

builds a release binary and wraps it as `target/bundle/MIDI Party.app` with the
icon and the bundled SoundFonts.

## Headless tools

```bash
cargo run -p mp-cli -- keys "n Right n Right Up Up Up Up n" --out song.ron
cargo run -p mp-cli -- render --sf2 assets/soundfonts/florestan-subset.sf2 --song song.ron --out song.mp3
```

## Layout

- `crates/mp-core` — song model, tempo/meter math, the `Command` enum, keymap, undo, command-line parser, `.mid` and `.ron` files.
- `crates/mp-audio` — real-time engine (cpal + rustysynth), sequencer, offline WAV/MP3 render. The audio callback is tested to allocate nothing.
- `crates/mp-app` — the egui window: tracks area, piano roll, lane strip, status bar, cheat sheet, command line, keymap editor.
- `crates/mp-cli` — `mp keys` / `mp render` for scripts and tests.

## Versions

The version number lives in one place, `version` in the root `Cargo.toml`, and
follows [semantic versioning](https://semver.org/).

Saved `.ron` songs carry a separate `format_version`. A file from a newer
format is refused with a clear message; raise `FORMAT_VERSION` in
`crates/mp-core/src/project.rs` only when older builds could no longer read
new files.

## Licensing note

MP3 export links LAME statically via `mp3lame-encoder` (LGPL-3.0). The test
SoundFont (`assets/soundfonts/florestan-subset.sf2`) is public domain and the
drum kit (`fluidr3-standard-kit.sf2`) is MIT; see the `LICENSE.txt` beside
them. The bundled font (Anonymous Pro) is under the SIL Open Font
License, see `assets/fonts/LICENSE.txt`. Everything else is MIT/Apache/Unlicense.
