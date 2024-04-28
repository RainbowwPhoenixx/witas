This collects notes on internal game types that haven't yet been translated into code.
This will be mostly random thoughts.
Adresses will be references to the game binary

# Input representation
To represent inputs, they use their own internal type.
To translate from a keycode to their type, they use some kind of lookup table (see 0x140344aca).

They have a global variable with the status of all the keys (indexed by their type). The values in this table represent the state of the key with flags:
    - Bit 1 is 1 if the key is down
    - Bit 2 is 1 if the key is being pressed
    - Bit 3 is 1 if the key is being released
This is mostly speculation through observation of the live values while pressing keys

Therefore, when you press a key, the value in the table will be:
    - 0b000 before the key press
    - 0b011 when it is pressed
    - 0b001 for as long as it is down
    - 0b101 when released
    - 0b000 after the release

The following values have been seen:
    - 0x7A -> Z
    - 0x10E -> LSHIFT

## Windows event handling
On the event handling side, the game only uses:
- WM_INPUT
- WM_LBUTTONDOWN/UP
- WM_RBUTTONDOWN/UP
to take player actions. All others are superfluous, so we use these to send events to the game.

WM_MOUSEMOVE is used for menu movement.
