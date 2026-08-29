# Echolocation mode

> **Living design note:** Reference this file before changing Echolocation, and update it in the same change whenever the mode's mechanics, controls, presentation, or technical behavior changes.

## Premise

Echolocation is an open-ended first-person exploration mode in a dark voxel environment. The world is not normally visible. The player learns its shape by sending out sound pulses, then survives an invisible pursuer that follows through the same rooms and corridors.

There is no victory condition. The purpose is to keep moving, orient from sound, and avoid being found.

## Player experience

- Move through the map using the normal first-person walking controls and mouse look.
- Hold `Space` or the left mouse button, then release to emit an echo pulse. A quick tap sends the normal pulse; holding it charges a longer-range pulse up to 160 units.
- Revealed geometry fades after a short time. Only faces exposed to open air render; faces buried against adjacent voxels remain hidden, so echoes do not reveal through solid ground or walls.
- The pulse has a cooldown. Its default speed is 10 and its default maximum range is 92.
- Charged pulses trade safety for information: the extra charge temporarily makes the pursuer faster for 3 seconds. The HUD warns `LOUD PULSE: IT IS CLOSER` while this effect is active.
- Walking produces close player footstep sounds and tiny non-reflecting echo waves from the player’s floor position. They have the same 2.2-unit range and surface-return behavior as the pursuer’s footstep waves.
- Reflections can create secondary pulses when echo strength is increased.
- `V` toggles the full-map debug view.
- `Tab` opens tuning controls: `[` / `]` change range, `-` / `=` change pulse speed, `,` / `.` change echo strength, and `R` restores the tuning defaults.
- `M` returns to the menu and `Escape` releases mouse capture.

## The invisible pursuer

One unkillable pursuer spawns on the reachable floor cell farthest from the seeded player start. It is entirely absent from the voxel world and echo-reveal data: it has no visible model, voxel, reflection, hitbox, or weapon interaction.

It continually rebuilds a navigation field from the player position and uses it to follow rooms and corridors rather than cutting through walls. It walks at a deliberately slow pace (currently 3.0 units per second). Reaching within 0.72 units of the player ends the run immediately.

The pursuer is only communicated through its trail and sound:

- Each step leaves a left/right pair of footprints just above the floor surface.
- Footprints are visible only when directly in the player’s line of sight. They fade visually by age and expire after 4 seconds.
- A quiet, spatialized invisible-footstep sound plays once per step pair (currently every 0.52 seconds).
- Every individual print emits a tiny, non-reflecting echolocation wave. It begins at that exact floor position, moves at speed 5, and travels at most 2.2 units. These waves use the same surface-return path as the player’s pulse, but are much smaller and cannot reveal the pursuer itself.

## Death and restart

On contact, simulation stops: the player cannot move or emit pulses, and the pursuer stops updating. A high-priority overlay reads:

```text
YOU WERE FOUND
R restart   M menu
```

`R` starts a fresh run with the same deterministic Echolocation seed, resetting the player camera, input state, world reveals, pursuer, footprints, and waves. `M` returns to the mode menu.

## Technical model

- The fixed mode seed is `0xEC40_10CA_7100_0001`, making the map and pursuer spawn reproducible.
- Echo impacts temporarily populate a reveal table keyed by voxel coordinate. Rendering consults that table but draws only faces adjacent to air.
- Player pulse reflections use the normal echo-wave system; pursuer step waves have a separate short-lived record and never reflect.
- Footprint decals and step-wave markers are rendered in overlay layers, independently of the hidden voxel body, while still respecting direct line of sight.
- The navigation field is shared infrastructure also used by zombie movement. It is built against the Echolocation walking profile and supplies valid corridor-following steps.

## Guardrails for future changes

- Preserve the pursuer’s invisibility: visible prints and footstep audio are its only intentional cues.
- Keep it out of the echo-visible voxel data and out of weapon interaction.
- Keep Echolocation open-ended unless the design explicitly adds an ending.
- Retain the deterministic seed restart behavior unless a new run-selection design replaces it.
- Add or update deterministic tests when changing spawn placement, pathing, reveal visibility, footsteps, waves, death, or restart.
