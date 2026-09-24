# Wade: design

Wade is an animated desk companion. He lives on a small touchscreen, reacts to touch with facial expressions and small actions, and hosts a few simple utilities: a timer first, then settings and weather. The project is written in Rust and runs on two targets.

| Target                       | Role                                                                                                                              |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Desktop (macOS; Linux later) | Development, debugging, and automated tests. No hardware needed.                                                                  |
| M5Stack CoreS3 Lite          | The real device: an ESP32-S3 microcontroller with a 2.0" 320×240 capacitive touchscreen, speaker, sensors, and a 200 mAh battery. |

## Goals

| Goal                         | In practice                                                                                                                          |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Platform-independent core    | All behavior lives in `wade-core`, which does not depend on hardware, an operating system, or a heap allocator.                      |
| Deterministic and testable   | Time and randomness are inputs. Any session can be reproduced exactly from its random seed and its list of timestamped input events. |
| Event-driven and power-aware | The platform sleeps until input arrives or until the core asks to be woken. Nothing runs on a fixed tick.                            |
| Desktop first                | Every feature runs and is tested on desktop before it runs on the device.                                                            |
| Small milestones             | Each milestone ends with something working on at least one target.                                                                   |

## Non-goals for now

Camera, microphones, motion sensors, text or speech from Wade, deep sleep, over-the-air updates, and localization are out of scope. Persistent storage and networking arrive in specific milestones (see [roadmap.md](roadmap.md)).

## Documents

| File                               | Contents                                                          |
| ---------------------------------- | ----------------------------------------------------------------- |
| [architecture.md](architecture.md) | Crate layout, the core API, time, events, effects, deadlines      |
| [character.md](character.md)       | Persona, face rig, expressions, animation, behavior rules         |
| [ui.md](ui.md)                     | Screens, navigation, touch handling, layout, rendering, the timer |
| [platforms.md](platforms.md)       | Desktop and CoreS3 Lite implementations, hardware, toolchain      |
| [testing.md](testing.md)           | Test layers, the test harness, snapshots, recording and replay    |
| [roadmap.md](roadmap.md)           | Milestones with scope and completion criteria                     |
| [decisions.md](decisions.md)       | Key decisions, the reasons for them, and rejected alternatives    |

## Glossary

| Term        | Meaning                                                                                                                             |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `no_std`    | Rust without the standard library. Only `core` is available (plus `alloc` if a heap is provided). Required for bare-metal firmware. |
| Embassy     | An async runtime for embedded Rust. It provides tasks, timers, and channels without an operating system.                            |
| esp-hal     | The Rust hardware abstraction layer for Espressif chips. It is `no_std` and integrates with Embassy.                                |
| Xtensa      | The CPU architecture of the ESP32-S3. It needs Espressif's Rust toolchain rather than a standard `rustup` target.                   |
| PSRAM       | External RAM (8 MB on this device). Larger but slower than the chip's 512 KB of internal SRAM.                                      |
| PMIC        | Power-management IC (an AXP2101 here). Handles battery charging and switches power to peripherals.                                  |
| IO expander | A chip (an AW9523B here) that adds GPIO pins over I²C. On this device it drives the reset and enable lines of several peripherals.  |
| DMA         | Hardware that copies data, such as a frame of pixels to the display, without using the CPU.                                         |
| Rgb565      | A 16-bit color format: 5 bits red, 6 bits green, 5 bits blue. The display's native format.                                          |
| Framebuffer | An in-memory image of the whole screen. Drawing goes into it, then it is sent to the display.                                       |
| Deadline    | The next time the core needs to run even if no input arrives. Reported by `App::next_deadline`.                                     |
| View        | Plain data describing what is on screen. The core produces it; `wade_core::render::draw` turns it into pixels.                      |
