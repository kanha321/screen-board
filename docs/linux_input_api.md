# Linux Input Event API (`evdev`) for drawing tablets

This guide explains how Linux handles drawing tablet inputs natively under the `evdev` system, including coordinates, pressure, and button state structures.

---

## 1. Struct `input_event` (C/Binary Level)

At the lowest level, reading from `/dev/input/event*` yields a stream of fixed-size binary structures defined in the Linux kernel (`linux/input.h`):

```c
struct input_event {
    struct timeval time; // 16 bytes on 64-bit systems (8s, 8us)
    __u16 type;          // 2 bytes
    __u16 code;          // 2 bytes
    __s32 value;         // 4 bytes
};
```

On a 64-bit Linux machine, the structure size is exactly **24 bytes**.

---

## 2. Event Types (`type`)

* **`0` (`EV_SYN`)**: Synchronization event. Indicates a bundle of inputs (e.g. coordinate change + pressure change) is complete and should be rendered.
* **`1` (`EV_KEY`)**: Key press / Button click.
  * `value = 1`: Key down
  * `value = 0`: Key up
  * `value = 2`: Key repeat (auto-repeat)
* **`3` (`EV_ABS`)**: Absolute coordinate updates (stylus).
* **`4` (`EV_MSC`)**: Miscellaneous metadata (like raw USB HID scancodes: `MSC_SCAN` / code 4).

---

## 3. Stylus Axis Codes (`code` on `EV_ABS`)

When the pen is hovered or dragged over the tablet surface, it fires absolute update events:

| Event Code | Event Name | Min / Max Values | Meaning |
| :--- | :--- | :--- | :--- |
| **`0`** | `ABS_X` | `0` – `40640` | Absolute horizontal coordinate |
| **`1`** | `ABS_Y` | `0` – `25400` | Absolute vertical coordinate |
| **`24`** | `ABS_PRESSURE` | `0` – `16383` | Pen tip down pressure sensitivity |
| **`26`** | `ABS_TILT_X` | `-127` – `127` | Left-to-right tilt angle of the pen |
| **`27`** | `ABS_TILT_Y` | `-127` – `127` | Bottom-to-top tilt angle of the pen |

---

## 4. Normalizing Coordinates & Drawing Physics

To render these inputs on a computer monitor, you must scale the values:

$$\text{X}_{\text{pixel}} = \frac{\text{ABS\_X}}{\text{MAX\_X}} \times \text{Width}_{\text{screen}}$$

$$\text{Y}_{\text{pixel}} = \frac{\text{ABS\_Y}}{\text{MAX\_Y}} \times \text{Height}_{\text{screen}}$$

### Pen Pressure Stroke Width:
To create realistic drawing physics, stroke width should vary dynamically:

$$\text{Width}_{\text{stroke}} = \text{Width}_{\text{min}} + \left(\frac{\text{ABS\_PRESSURE}}{\text{MAX\_PRESSURE}}\right) \times (\text{Width}_{\text{max}} - \text{Width}_{\text{min}})$$
