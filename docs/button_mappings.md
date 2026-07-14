# Gaomon WH851 Button Mappings

This document records the exact physical buttons, raw hardware scancodes, and default keyboard outputs of the Gaomon WH851 tablet (connected via Bluetooth) as detected on Linux.

---

## 1. Stylus Pen Buttons

The stylus pen has two side switches. When clicked close to the tablet surface, they emit standard keyboard keypresses on the virtual keyboard handler:

* **Lower Button (Closer to Tip)**:
  * **Raw Scancode (Hex)**: `70005` (USB HID Usage Keyboard `b` and `B`)
  * **Linux Keycode / Name**: `48` (`KEY_B`)
  * **Default Action**: Toggles paintbrush tool in most drawing software.

* **Upper Button (Farther from Tip)**:
  * **Raw Scancode (Hex)**: `70008` (USB HID Usage Keyboard `e` and `E`)
  * **Linux Keycode / Name**: `18` (`KEY_E`)
  * **Default Action**: Toggles eraser tool in most drawing software.

---

## 2. Tablet Express Keys (Buttons on the Tablet Bezel)

The tablet features 8 physical press keys arranged vertically on the left panel. These buttons send a mix of single keys and multi-key combinations:

| Express Key | Scancode(s) (Hex) | Key Code(s) | Default Key Combination |
| :--- | :--- | :--- | :--- |
| **Button 1 (Top)** | `70005` | `48` | `B` |
| **Button 2** | `70008` | `18` | `E` |
| **Button 3** | `7002C` | `57` | `Space` (Hand/Pan tool) |
| **Button 4** | `7000C` | `23` | `I` (Color eyedropper tool) |
| **Button 5** | `7002F` | `26` | `[` (Decrease brush size) |
| **Button 6** | `70030` | `27` | `]` (Increase brush size) |
| **Button 7** | `700E0` + `70008` | `29` + `18` | `Ctrl + E` |
| **Button 8 (Bottom)**| `700E0` + `700E2` + `7001D` | `29` + `56` + `44` | `Ctrl + Alt + Z` (Undo / Step Backward) |

---

## 3. The Mechanical Dial & Center Button

* **Center Button**: Used internally by the tablet hardware to cycle through the dial's modes (e.g., Zoom, Scroll, Brush size). By default, it does **not** emit any keyboard events to the PC.
* **Dial Wheel Rotation**: Depending on the active mode selected by the center button, rotating the dial will emit keypresses corresponding to that function (e.g. `Ctrl + +` / `Ctrl + -` for zoom, or `[` / `]` for brush size).
