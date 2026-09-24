# Roadmap

| Milestone | Target | Summary | Requires |
|---|---|---|---|
| [M1 Desktop skeleton](#m1-desktop-skeleton) | Desktop | Core types, Wade's eyes with every expression, test harness, record and replay | Nothing |
| [M2 Timer](#m2-timer) | Desktop | Timer screen, navigation, chime | M1 |
| [Hardware spike](#hardware-spike) | Device | Throwaway firmware proving display, touch, audio, and timing | The device |
| [M3 Device port](#m3-device-port) | Device | `wade-cores3` runs everything from M2 | M2, spike |
| [M4 Character](#m4-character) | Both | Look and motion tuned on the device, props and idle activities, personality | M3 |
| [M5 Settings](#m5-settings) | Both | Launcher, settings screen, persistent settings | M3 |
| [M6 Power](#m6-power) | Both | Display sleep, wake on approach, battery status | M5 |
| [M7 Weather](#m7-weather) | Both | Wi-Fi, weather fetch, weather screen | M5 |

M1 and M2 need no hardware. The hardware spike can start as soon as the device arrives, in parallel with M1 or M2. M4 and M5 are independent of each other.

## M1 Desktop skeleton

Scope:

| Area | Work |
|---|---|
| Workspace | `wade-core` and `wade-desktop` crates; `wade-cores3` excluded (see [architecture.md](architecture.md#crates)) |
| Core | `Instant`, `Event`, `App`, `Output`, `View`, `render::draw`, `TouchTracker` (taps), `layout`, seeded PRNG |
| Wade | Eyes on the pose rig; every expression; the animation layers; M1 behavior rules from [character.md](character.md#m1) |
| Desktop | Window, mouse-to-touch mapping, deadline-driven loop, `--seed`, `--record`, `--replay`, `--time-scale` |
| Tests | Harness; behavior, invariant, and snapshot tests; the `no_std` and no-`alloc` checks; recordings with state hashes |
| CI | GitHub Actions workspace job (see [testing.md](testing.md#ci)) |

Behavior tests:

| Test | Expectation |
|---|---|
| Start | Buddy screen, Neutral |
| Tap on Wade | Happy |
| 1,999 ms after the tap | Still Happy |
| 2,000 ms after the tap | Neutral |
| Second tap while Happy | The 2 s restarts from the second tap |
| Tap outside Wade | No change |
| Touch down on Wade, move off, lift | No tap, no change |
| Blinking | With a fixed seed, a blink starts within 6 s of startup and lasts 120 ms |
| Blink rhythm | Blinks are 2 to 6 s apart, or doubled 300 ms after one ends |
| Still | No frame requests while nothing moves |

Done when:

| # | Criterion |
|---|---|
| 1 | All [checks](testing.md#checks) pass |
| 2 | `cargo run -p wade-desktop` shows Wade, who blinks every 2 to 6 s, is Happy for 2 s when clicked |
| 3 | The desktop process uses negligible CPU while Wade is still |
| 4 | A session recorded with `--record` replays with `--replay` with every state hash matching |
| 5 | CI runs the checks on every push |

## M2 Timer

Scope: the `Screen` enum, the apps button, the timer state machine and screen as specified in [ui.md](ui.md#timer), seven-segment digits, pressed button styles, `Effect::Chime` with the tone sequence in `wade_core::sound`, desktop audio through `rodio`, and Wade's Proud reaction after a dismissed timer.

Behavior tests:

| Test | Expectation |
|---|---|
| Start | Running with `ends_at = now + set` |
| Rounding | With 299,001 ms remaining the display shows 05:00; with 500 ms remaining it shows 00:01 |
| Pause, then resume | Remaining time is preserved across the pause |
| Completion | At `ends_at`: Done, one `Chime`, screen switches to Timer |
| Chime repeats | While Done, a `Chime` at `ends_at + 10 s`, `+ 20 s`, …, 10 in total; none after that |
| Dismiss stops chimes | Dismissing after the third chime: no further `Chime` |
| Completion while on Buddy | Same as above; the timer completes on time without the Timer screen being visible |
| Completion mid-touch | Touch down on Wade, the timer completes, lift: no tap on the Timer screen, no change |
| Wade while hidden | On the Timer screen, Wade requests no frame deadlines and never sets `redraw` |
| Navigation | Leaving and returning to the Timer screen does not change the timer |
| Deadlines on Timer screen while Running | The next deadline is the next second boundary or `ends_at`, whichever is sooner |
| Deadlines on Buddy while Running | The timer contributes only `ends_at` |
| Bounds | −1m does nothing at 1:00; +1m does nothing at 99:00 |
| Dismiss | Ready with the same `set`; screen returns to Buddy; Wade is Proud for 2 s |
| Back while Done | Same as Dismiss |

Snapshots: the Timer screen in each state, and each button pressed.

Done when:

| # | Criterion |
|---|---|
| 1 | All checks pass |
| 2 | On desktop: set 1:00, start, go back to Wade, and at 1:00 the chime plays and the Timer screen shows Done |
| 3 | With `--time-scale 10`, a 5:00 timer finishes in 30 s of real time |

## Hardware spike

A throwaway firmware in `spikes/cores3-spike/`, outside both workspaces. `spikes/` must be listed in the root workspace's `exclude` (see [architecture.md](architecture.md#crates)). Its purpose is to retire hardware risk before the port.

| # | Goal |
|---|---|
| 1 | Confirm the unit has the parts in [platforms.md](platforms.md#hardware), especially the proximity sensor and battery. If not, switch to the standard CoreS3. |
| 2 | Install the toolchain; flash a program that logs over USB serial |
| 3 | Configure the PMIC and IO expander; turn the display on |
| 4 | Fill the screen with solid colors; measure full-frame flush time |
| 5 | Measure the flush time of partial windows (the blink and expression-change rectangles in [ui.md](ui.md#partial-flush)), and try SPI clocks above 40 MHz |
| 6 | Draw `embedded-graphics` shapes and text |
| 7 | Read touch points; confirm they match display coordinates and orientation. Check whether the interrupt line is usable (believed to be routed through the AW9523B). |
| 8 | Play a tone through the amplifier and speaker |
| 9 | Find how display brightness is controlled (believed to be an AXP2101 LDO voltage) |
| 10 | Decide where the framebuffer lives (internal SRAM or PSRAM), with M7's Wi-Fi and TLS memory needs in mind, and confirm DMA from it works |

Output: `docs/hardware-notes.md`, recording initialization sequences, pin and register details, and measured numbers (flush times, memory use). It ends with a decision on whether M3 needs `render::damage`. Done when all ten goals are demonstrated and written up.

## M3 Device port

Scope: the `wade-cores3` crate with the app, touch, and audio tasks from [platforms.md](platforms.md#tasks); framebuffer rendering and flushing, with `render::damage` and partial flushes if the spike calls for them ([ui.md](ui.md#partial-flush)); seeding from the hardware random number generator; the M2 feature set running unchanged from `wade-core`; the CI firmware job.

Done when:

| # | Criterion |
|---|---|
| 1 | The desktop scenarios from M1 and M2 work on the device |
| 2 | Blinks and expression transitions render at 20 fps or better (measured) |
| 3 | The time from touch to visible response is under 100 ms (measured) |
| 4 | The device runs for 1 hour without panicking or drifting (a 60:00 timer finishes within a second of a phone stopwatch) |
| 5 | `wade-core` needed no changes for the port, or every change is also covered by desktop tests |
| 6 | CI builds the firmware on every push |

## M4 Character

Scope: tune Wade's look and motion on the real screen; implement props and idle activities from [character.md](character.md); finalize and implement the M4 behavior rules.

Tests: behavior tests for the activity scheduler, the multi-tap Proud rule, and the hubris beat, using fixed seeds; a snapshot of each activity's key frame.

Done when:

| # | Criterion |
|---|---|
| 1 | All expressions and activities are visible and readable on the device at arm's length |
| 2 | Snapshots are updated and reviewed |
| 3 | The design passes the legibility constraints in [character.md](character.md#appearance) |

## M5 Settings

Scope: the Launcher screen; a Settings screen; persistence.

Planned settings: display brightness (4 levels), chime on or off (off silences the first chime and every repeat), and the default timer duration. Confirm this list at the start of the milestone.

Persistence: the core defines `Settings` with `encode` and `decode` for a fixed binary layout that starts with a version byte. `decode` falls back to defaults for missing, corrupt, or unknown-version data. The core emits `Effect::SaveSettings(settings)` once settings have stopped changing for 2 s, so stepping through brightness levels writes flash once, not on every tap. The debounce is an ordinary core deadline; leaving the Settings screen saves at once. The platform stores the bytes (desktop: a file in the user's config directory; device: flash through `esp-storage` and `sequential-storage`). At startup the platform loads the bytes, decodes them, and passes the result to `App::new`.

Done when: settings survive a restart on both platforms, and corrupt stored data yields defaults rather than a crash (tested).

## M6 Power

Scope:

| Area | Work |
|---|---|
| Events | `Power(PowerStatus)` with battery percentage, charging, and external power; `Proximity(Near \| Far)` |
| Effects | `DisplayPower(bool)`, plus the existing `SetBrightness` |
| Behavior | After a period without touch or proximity (a setting), Wade becomes Sleepy, falls asleep, and then the display turns off. A touch or an approach turns it back on, with Surprised then Neutral. |
| Timer | Keeps running with the display off. On completion the chime plays and the display turns on. |
| Battery | Battery level on the Settings screen. A small indicator on the Buddy screen when running on battery. |
| Desktop | Keyboard keys simulate power and proximity events |
| Device | The CPU idles whenever no task is ready to run. Measure current draw with the display on and off. |

Out of scope: deep sleep, which loses RAM and therefore all state.

Open questions to settle at the start: whether the touch interrupt is usable through the IO expander or polling must continue; whether ambient light should drive brightness automatically.

## M7 Weather

Scope:

| Area | Work |
|---|---|
| Network | Wi-Fi through `esp-radio` and `embassy-net`. Credentials come from build-time environment variables (`WADE_WIFI_SSID`, `WADE_WIFI_PASSWORD`) and are never committed. |
| Source | A weather API that needs no key, such as Open-Meteo. Location is a build-time latitude and longitude. |
| Core | A Weather screen in the Launcher showing temperature, a condition icon drawn from shapes, and "updated N min ago". The core emits `Effect::FetchWeather` when the screen opens and the data is more than 30 minutes old. |
| Events | `Weather(WeatherUpdate)`, carrying either a report or a failure reason |
| Parsing | A pure function in `wade-core` (using `serde-json-core`) turns the response body into a `WeatherReport`, so parsing is testable on desktop. The platform fetches bytes and calls it. |
| Desktop | A blocking HTTP client on a background thread |

No wall-clock time is needed; "updated N min ago" uses `Instant`.

Risk: the API uses HTTPS, and TLS on `no_std` means an additional crate (such as `esp-mbedtls` or `embedded-tls`) and extra memory. Spike this first, before building the screen. Fallback: Open-Meteo is believed to also serve plain HTTP, which avoids TLS entirely at the cost of an unencrypted request (it carries only a location). Confirm before relying on it.

Risk: memory. The Wi-Fi stack, its heap, and TLS all need internal RAM, competing with the framebuffer if it lives there. The spike's framebuffer decision should already account for this; if it does not, M7 starts by re-measuring.

Open questions: units (a setting, °C or °F); which weather conditions get icons.

## Future

Candidates, in no particular order: a scrolling text readout on the Buddy screen for quips and status; wall-clock time from SNTP or the real-time clock; Wi-Fi setup on the device through a captive portal instead of build-time credentials; deep sleep with timed wake; richer personality behaviors.
