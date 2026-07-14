# Remapping Tablet Keys via udev (hwdb)

Because the Gaomon WH851 tablet sends standard keyboard strokes (like `B`, `E`, and `Ctrl+Alt+Z`), using the buttons normally will interfere with your standard typing or conflict with hotkeys in other applications.

This guide explains how to write a custom hardware database (`hwdb`) rule on Linux to intercept these scancodes specifically for the tablet and map them to unassigned function keys (`F13`–`F21`).

---

## Step 1: Create the Config File

Create a new file `/etc/udev/hwdb.d/99-gaomon-wh851.hwdb` with root privileges:

```bash
sudo nano /etc/udev/hwdb.d/99-gaomon-wh851.hwdb
```

Paste the following configuration:

```hwdb
# Gaomon WH851 Bluetooth Tablet Remap
evdev:input:b0005v256Cp8251*
 KEYBOARD_KEY_70005=f13
 KEYBOARD_KEY_70008=f14
 KEYBOARD_KEY_7002c=f15
 KEYBOARD_KEY_7000c=f16
 KEYBOARD_KEY_7002f=f17
 KEYBOARD_KEY_70030=f18
 KEYBOARD_KEY_700e0=f19
 KEYBOARD_KEY_700e2=f20
 KEYBOARD_KEY_7001d=f21
```

### Format Rules:
1. The match line `evdev:input:b0005v256Cp8251*` targets Bluetooth bus (`b0005`), Vendor `256C`, and Product `8251`.
2. Every mapping line **must** start with exactly **one leading space** (e.g. ` KEYBOARD_KEY_...`).
3. The scancode is hexadecimal, lowercase (e.g., `70005`).
4. The keycode is lowercase (e.g., `f13`).

---

## Step 2: Apply the Remap

Update the system's hardware database and tell `udev` to trigger the changes on your active inputs:

```bash
sudo systemd-hwdb update
sudo udevadm trigger
```

---

## Step 3: Verify the Changes

Run `evtest` on `/dev/input/event259` and press the express keys. You should now see them log events like:

```text
Event: type 1 (EV_KEY), code 183 (KEY_F13), value 1
```

Instead of sending letters to your focused text field, they will emit safe `F13`–`F21` keys that your overlay app can listen for.
