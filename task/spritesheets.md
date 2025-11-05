
## 🎯 Goal

Implement support for an animated 8-direction player sprite in a Rust raycaster using **`winit`** + **`pixels`**.
The server handles game logic; clients render other players as sprites based on their animation state and angle relative to the camera.

---

## 🧠 Concept Overview

Each player is represented as a 2D sprite billboard.
The correct sprite frame depends on:

* **Animation state** (idle, walking, shooting, etc.)
* **View angle** (which direction the camera sees the player from)
* **Animation timing** (which frame in the sequence)

---

## 🧩 Player Render State

Each player should have a render-related structure like this (computed client-side):

```rust
struct PlayerRenderState {
    animation_state: AnimationState,  // Idle, Walking, Shooting, etc.
    facing_angle: f32,                // Direction player is facing
    current_frame: usize,             // Current animation frame index
    frame_timer: f32,                 // Time accumulator for frame changes
}
```

The render system updates this every frame and selects the appropriate sprite.

---

## 🖼️ Spritesheet Details

The spritesheet has the following structure:

* Frame size: **91×92 pixels**
* 1-pixel separator between frames
* Background color: **#00FFFF** (cyan) → should be treated as **transparent**
* Layout:

  * **Idle animation**: 1 row, 8 directions (starting at x=1, y=34)
  * **Walking animation**: 8 rows × 4 frames (starting at x=1, y=142)

    * Each row = same animation from a different viewing angle
    * Each next row starts 93 pixels lower
  * Extra text and decorations on sheet can be ignored

You can treat the image as a grid of sprite rectangles and slice them accordingly.

---

## 📥 Spritesheet Loading

At game initialization:

1. Load the spritesheet image (using `image` crate, e.g. `image::open("sprites.png")`).
2. Iterate through rows/columns based on known coordinates.
3. Extract each frame as a subimage (use `image::SubImage` or `crop_imm`).
4. Convert cyan (`#00FFFF`) pixels to transparent.
5. Store frames in a lookup structure:

```rust
struct SpriteSheet {
    idle: [Frame; 8],                      // 8 directions
    walk: [[Frame; 4]; 8],                 // 8 directions × 4 frames
}

struct Frame {
    pixels: Vec<u8>, // RGBA buffer for pixels crate
    width: u32,
    height: u32,
}
```

This precomputes all frames into memory for fast rendering.

---

## 🧮 Direction Calculation

When rendering another player:

1. Compute the **angle from camera to player**:

   ```rust
   let angle_to_player = (player.pos - camera.pos).angle();
   ```
2. Compute **relative angle** between player’s facing angle and camera:

   ```rust
   let relative_angle = normalize_angle(angle_to_player - player.facing_angle);
   ```
3. Determine **direction index (0–7)**:

   * Divide the full circle (360°) into 8 equal 45° sectors.
   * Example:

     ```
     0 = front
     1 = front-right
     2 = right
     ...
     7 = front-left
     ```

This gives which row of the sprite to use.

---

## 🎞️ Animation Timing

Each animation has multiple frames that cycle at a fixed rate (e.g. 150 ms per frame).

Per update tick:

```rust
player.frame_timer += delta_time;
if player.frame_timer > frame_duration {
    player.frame_timer = 0.0;
    player.current_frame = (player.current_frame + 1) % num_frames;
}
```

Idle animation has only one frame, while walking uses four.

---

## 🕹️ Animation State Logic

At render time:

```rust
let animation = if player.velocity.length() > 0.1 {
    AnimationState::Walking
} else {
    AnimationState::Idle
};
```

Then look up:

```rust
let frame = match animation {
    AnimationState::Idle => sprites.idle[direction_index],
    AnimationState::Walking => sprites.walk[direction_index][player.current_frame],
};
```

---

## 🧩 Full Conceptual Flow

```
load_spritesheet()
    → slice all frames
    → store in SpriteSheet structure

render_players()
    for each remote player:
        determine AnimationState
        determine direction_index (relative to camera)
        advance animation frame
        get correct sprite from SpriteSheet
        draw sprite to Pixels frame buffer
```

---

## ✅ Summary

* Load and preprocess all sprite frames **once** at startup.
* Determine animation state + view angle each frame.
* Advance animation timer to select current frame.
* Render the selected frame using `pixels`.
* Use color key transparency for `#00FFFF`.

This matches the approach used by Doom/Wolfenstein-style raycasters and works efficiently with `winit` + `pixels`.
