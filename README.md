# witas - The Witness tas tools

## Features
### Central features
- Playing a tas from a script input file
- Extremely minor modding used to make the physics consistent (fixed at 60Hz)
- A simple GUI to:
    - Control TAS playback (Play, Stop, Fast-Forward, Frame by frame)
    - Some information like current playback tick and player pos/angles
    - Various other configuration/tools

### Additionnal features
- During TAS playback, the player position is recorded and the displayed physically in the world with little spheres
- Disables game slowing down when alt-tabbing so you can actually use the GUI without messing up the playback
- Useful keybinds (only with the game focused):
    - P: quick tas replay
    - N: noclip toggle
    - J/K: position saving/restoring (very useful while TASing snipes)
    - E: puzzle debug toggle

## Setting it up, usage and debugging
### Linux
- Build the project
- Copy libwitness_tas.so to the game folder
- Change the steam launch command to be `LD_PRELOAD="libwitness_tas.so" %command%`
- Start the game and the GUI, you should be good to go

### Windows
- Build the project
- Put the executable (`witness_tas_controller.exe`) and the library (`witness_tas.dll`) in the same directory
- Start the game and the executable
- Click on "Inject & Connect" in the GUI

### Basic usage
- Create a folder called "tas" in the game folder, you can place your tas scripts in there
- Create a new file in the folder and start writing your tas
- Play the TAS with the GUI (specify the correct filename!)
- Edit your TAS, replay, edit, replay, etc

### Troubleshooting
The injected library produces a log file called witness_tas.log to help troubleshoot issues.


## Planned features
- Savestates
- In my wildest dreams, we would also leverage pathfinding and automatic puzzle solving
- Puzzle solving hud for snipes, to help you see what you're doing
- Scan instead of adresses
- Windows support, with automatic DLL injection when opening GUI
- More stuff for the GUI:
    - Config stuff:
        - TAS folder (currently forced to "game dir/tas")
        - number of decimals in the pos/ang display
- Freecam, to watch the TAS from any world point (useful to watch a panel while tasing a snipe)

## Todo
- Document the script format
- Automatically release buttons at the end
- Handle resolution:
    - Make mouse movement independent of res?
    - Make res a parameter of the script?
- Add vertical smoothing to the trace
- RE:
    - WorldToScreen function?
    - drawing debug stuff
    - HOW CLICK PUZZLES WHILE MOVING
- stop the tas when pressing escape
- so injection on linux

# Contributing
The project is open to contributions. Here's a brief rundown of how it works.

The tas tool is made up of two parts:
- The actual TAS tool, running inside the game
- The GUI, a separate program

The first one is a library (.dll/.so) injected into the game(via dll injection on windows, and `LD_PRELOAD`ing on linux).
Its job is to hook game functions, and instrument the game such that we can play tasses and that they are consistent
(plus some qol stuff to help the tassing process). It also opens a websocket on localhost in order to communicate with
a controller (in this case, the GUI).

The second one allows for easy control of the TAS tool.

The two components communicate with a protocol defined in src/communication.rs. Messages are serialized to json. Any
program connecting to the socket using that protocol can make the tool run a TAS. For example, this could be used for
a brute-forcing tool.
