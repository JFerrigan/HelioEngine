# Echolocation mode

> **Living design note:** Reference this file before changing Echolocation, and update it in the same change whenever the mode's mechanics, controls, presentation, or technical behavior changes.

## Premise

Echolocation is an open-ended first-person exploration mode in a dark voxel environment. The world is not normally visible. The player learns its shape by sending out sound pulses, then survives an invisible pursuer that follows through the same rooms and corridors.

There is no victory condition. The purpose is to keep moving, orient from sound, and avoid being found.

## Sound-activated receiver door

The first interactive puzzle occupies the starting room. A closed bulkhead at `x = -21` seals its east passage except for a three-voxel-wide, four-voxel-high doorway filled by a movable door. A one-voxel floor receiver at approximately `(-36, 1, 0)` is 15 voxels from the door plane. A straight signal pipe replaces the floor blocks between them without obstructing the walking route.

- Activation requires a propagated sound impact on the receiver voxel. Primary and reflected player pulses, player step waves, and pursuer step waves can all trigger it; merely standing nearby cannot.
- A hit makes the receiver emit for 3 seconds. A new hit while it is emitting extends that interval to at least 3 seconds after the new hit. A hit after shutoff creates a separate interval, so the already-travelling gap is preserved.
- The signal moves along the ordered pipe at 6 voxels per second. Each pipe sample is powered when the receiver was emitting at `current time - distance / 6`, producing visible leading and trailing edges. The 15-voxel route therefore delays the door by 2.5 seconds and preserves the 3-second crossing window.
- The door clears from the shared voxel world on the delayed rising edge and is restored on the falling edge. When restored over the player or pursuer, that entity moves to the nearer predefined clear position; equal distances use its current side, then the starting-room side.
- Closed door voxels block walking, pursuer navigation, sight, and newly emitted waves. Opening removes those voxels for all four systems. Waves cache their path at emission, so waves already in flight retain the path computed before a transition.
- Idle receiver, pipe, and door geometry follows ordinary echo reveal and full-map debug rules. Powered pipe samples, the emitting receiver, and the powered endpoint are cyan-blue and self-lit, but still require direct line of sight. Their colored layer sits above geometry and footprints and below searching static and HUD overlays.
- Receiver clicks and door mechanisms use the same listener-relative pan and distance attenuation convention as pursuer footsteps.

## Player experience

- Move through the map using the normal first-person walking controls and mouse look.
- Hold `Space` or the left mouse button, then release to emit an echo pulse. A quick tap sends the normal pulse; holding it charges a longer-range pulse up to 160 units.
- Revealed geometry fades after a short time. Only faces exposed to open air render; faces buried against adjacent voxels remain hidden, so echoes do not reveal through solid ground or walls.
- The pulse has a cooldown. Its default speed is 10 and its default maximum range is 92.
- Charged pulses trade safety for information: their longer-ranging wave can reach the invisible pursuer and reveal where the pulse began. They do not change pursuer speed.
- Walking produces close player footstep sounds and tiny non-reflecting echo waves from the player’s floor position. A pursuer that is within 12 horizontal units with a direct line of sight can hear the step and investigate it.
- Reflections can create secondary pulses when echo strength is increased.
- `V` toggles the full-map debug view.
- `Tab` opens tuning controls: `[` / `]` change range, `-` / `=` change pulse speed, `,` / `.` change echo strength, and `R` restores the tuning defaults.
- `M` returns to the menu and `Escape` releases mouse capture.

## The invisible pursuer

One unkillable pursuer spawns on the reachable floor cell farthest from the seeded player start. It is entirely absent from the voxel world and echo-reveal data: it has no visible model, voxel, reflection, hitbox, or weapon interaction.

It wanders through seeded reachable floor cells, sometimes pausing, and never navigates from the live player position. A player pulse alerts it only when that wave expands across its position; reflected waves preserve the original pulse location. Hearing a pulse or qualifying footstep gives it an 8-second investigation target. It paths to that remembered place, then searches nearby reachable cells until the timer ends or newer noise replaces it. It walks at a deliberately slow pace (currently 3.0 units per second). Reaching within 0.72 units of the player ends the run immediately.

The pursuer is only communicated through its trail and sound:

- Each step leaves a left/right pair of footprints just above the floor surface.
- Footprints are visible only when directly in the player’s line of sight. They fade visually by age and expire after 4 seconds.
- A quiet, spatialized invisible-footstep sound plays once per step pair (currently every 0.52 seconds).
- Every individual print emits a tiny, non-reflecting echolocation wave. It begins at that exact floor position, moves at speed 5, and travels at most 2.2 units. These waves use the same surface-return path as the player’s pulse, but are much smaller and cannot reveal the pursuer itself.
- While it has an active sound target, the HUD reads `IT HEARD YOU — SEARCHING`. Screen corruption follows a five-step proximity scale: distant searching begins with dim near-black edge flecks; middle ranges add gray reticle/HUD twitch and scattered ASCII replacement; close ranges add dense charcoal static and horizontal tears; point-blank searching heavily corrupts the ASCII view while remaining navigable. These are deterministic render-only effects and do not move the camera or affect input.
- Searching also carries a subdued, pursuer-panned dead-radio noise bed. Deterministic gray-noise crackles begin at the second proximity tier and become more frequent and stronger as it closes, but remain below pursuer footsteps. Both the bed and bursts stop on investigation expiry, death, restart, and leaving the mode.

## Death and restart

On contact, simulation stops: the player cannot move or emit pulses, and the pursuer stops updating. A high-priority overlay reads:

```text
YOU WERE FOUND
R restart   M menu
```

`R` starts a fresh run with the same deterministic Echolocation seed, resetting the player camera, input state, world reveals, pursuer behavior and remembered noise, footprints, waves, search effect, and the closed inactive receiver puzzle. `M` returns to the mode menu. Puzzle simulation time freezes on death and mode exit discards the run state normally.

## Technical model

- The fixed mode seed is `0xEC40_10CA_7100_0001`, making the map and pursuer spawn reproducible.
- Echo impacts temporarily populate a reveal table keyed by voxel coordinate. Rendering consults that table but draws only faces adjacent to air.
- Player pulse reflections use the normal echo-wave system; pursuer step waves have a separate short-lived record and never reflect.
- Footprint decals and step-wave markers are rendered in overlay layers, independently of the hidden voxel body, while still respecting direct line of sight.
- Receiver output is stored as emission intervals. Intervals are pruned only after their delayed trailing edges pass the door, allowing multiple independent signal bands to coexist on the route.
- The navigation field is shared infrastructure also used by zombie movement. It is built against the Echolocation walking profile and supplies valid corridor-following steps.

## Guardrails for future changes

- Preserve the pursuer’s invisibility: visible prints and footstep audio are its only intentional cues.
- Keep it out of the echo-visible voxel data and out of weapon interaction.
- Keep Echolocation open-ended unless the design explicitly adds an ending.
- Retain the deterministic seed restart behavior unless a new run-selection design replaces it.
- Add or update deterministic tests when changing spawn placement, pathing, reveal visibility, footsteps, waves, death, or restart.
