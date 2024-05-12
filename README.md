# witas - The Witness tas tools

## Features
### Central features
- Playing a tas from a script input file
- Extremely minor modding used to make the physics consistent (fixed at 60Hz)
- A simple GUI to:
    - Control TAS playback (Play, Stop, Fast-Forward)
    - Some information like current playback tick and player pos/angles
    - Various other configuration/tools

### Additionnal features
- During TAS playback, the player position is recorded and the displayed physically in the world with little spheres
- Disables game slowing down when alt-tabbing so you can actually use the GUI without messing up the playback
- Useful keybinds (only with the game focused):
    - P: quick tas replay
    - N: noclip toggle
    - J/K: position saving/restoring (very useful while TASing snipes)

## Planned features
- Frame by frame
- Savestates
- In my wildest dreams, we would also leverage pathfinding and automatic puzzle solving
- Puzzle solving hud for snipes, to help you see what you're doing
- Scan instead of adresses
- Windows support, with automatic DLL injection when opening GUI
- More stuff for the GUI:
    - Config stuff:
        - TAS folder (currently forced to "game dir/tas")
        - number of decimals in the pos/ang display
    - "About" tab with version, etc
- Freecam, to watch the TAS from any world point (useful to watch a panel while tasing a snipe)
- Make trace a different color when in solve mode
- Add looping to make the tas play on a loop

## Todo
- Document the script format
- Automatically release buttons at the end
- Handle resolution:
    - Make mouse movement independent of res?
    - Make res a parameter of the script?

# Contributing
The project is open to contributions. 

TODO: brief explanation of how it works.
