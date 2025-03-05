# Tool usage

This document outlines the General usage of the tool, the script format, and also had a bunch of tips and tricks I picked up or added to the tool that may be useful to others.

## General usage
### Basic usage
The TAS tool UI starts on the "Playback" tab, where the main controls live. At the bottom is a textbox where the name of the tas script file can be entered. By default, this file should be placed in the game files, in the `tas` directory.

Then you can press play to try playing the TAS. Any errors are reported in the "Info" section at the top.

If no errors occur, then the TAS starts playing and the "Infos" section updates with real-time data from the game.

The "Skip to tick" value is used fast-forward the TAS until the specified tick. This is especially useful for long running scripts and is achieved by skipping the draw calls and running the game logic as fast as possible. The actual speedup depends on your specs, I get around 30x on my machine.

The "Pause at tick" value is used to pause the TAS when it reaches the specified tick. This is useful when you want to examine what happens at a slower pace. You can then press the "Next frame" button to step through the TAS, or press "Play" to resume regular playing.

### Shortcuts
The tool adds a number of keyboard shortcuts to the game, to make routing and TAS dev easier:
- P: Replay the last played TAS
- ESC: Stop the TAS
- N: noclip
- E: Puzzle debug
- J/K: Save/Restore a position

### Trace
During TAS playback, the tool records the position history of the player, and by default displays the last 100 positions as green spheres.

In the "Trace" tab, you are able to change the way the sphere show, such as their radius or vertical offset. These options come in handy when the ground texture is not at the same height as the collision, and so the spheres may end up in the ground.

You may teleport to any of the recorded points (even the ones not shown) by selecting the tick in the "Teleport" section and clicking "Teleport" or checking "Continuous". This last option will teleport you to the tick anytime you change the value in the text box.

The tool also records attempts to click on a puzzle, and displays a little blue sphere in the direction of the click. In combination with the teleport feature, this allows you to adjust your angles more easily when trying to click a faraway puzzle.

Similar to the trace, you can adjust the click indicator's distance and size to fit the situation.

## Script format
A script is split in two sections: the header and the actual inputs.

At any point in the script, a double slash (`//`) can be used to write a comment.

### Header
The header contains two lines:
#### The version line
This line indicates the script version. This is increased when a breaking change is introduced to the script format.
Currently, only version 0 exists.

#### The start line
This line indicates the start mode of the TAS.

- `start newgame` starts the tas from a blank save. This does not currently reset the FOV.
- `start save <save name>.witness_campaign` starts the tas from the given save name.

### Inputs
After the header, the actual inputs are provided. Each line corresponds to one "instruction" under the following format:

`tick>buttons|look angles|tools`

A field can be ommited by leaving it empty, which means it will keep having the same value as a previous instruction. The pipes (`|`) can also be ommited if all the fields after it are empty

#### Tick field
The tick is a number specifying the tick (aka frame) that the instruction will run on. It can be absolute, which means it will run after the number of frames has elapsed from the start of the TAS. It can also be relative by adding a `+` before the number, in which case it will after the number of frames has elapsed from the previous instruction.

Absolute and relative tick types can be mixed.

Example:

```
5>     // will run on tick 5
7>     // will run on tick 7
+3>    // will run on tick 10
+2>    // will run on tick 12
15>    // will run on tick 15
```

#### Buttons field
This field is used to press buttons, and therefore control clicks and movement. An uppercase letter indicates that the button should be pressed from now on. A lowercase letter means the button should stop being pressed.

- `U`/`u`: up aka forwards
- `D`/`d`: down aka backwards
- `L`/`l`: left
- `R`/`r`: right
- `S`/`s`: sprint
- `E`/`e`: escape aka menu

The only exception is left/right click to enter/leave focus mode and click on puzzles.
- `P`: enter focus mode and click on puzzles (left click)
- `p`: leave focus mode (right click)

#### Look angles
This field consists of two numbers describing how much to move the mouse up/down and right/left. This field keeps the same value until it is reset to 0.

Example:

```
// Look up-right for 5 frames
5>||30 30
+5>|0 0

// Enter puzzle mode and move cursor down for 10 frames
20>P
+1>|0 -20
+10>|0 0
```

#### Tools
Tools are special commands used to TAS more easily, outside of the actual inputs.

Currently the only tool is `setpos <x> <y> <z> <yaw> <pitch>`, to set the precise positions and angles of the player. Its use in TAS is not legitimate, and is a tool that was created for the purpose of making segmenting and stitching easier.

```
5>||setpos 149.79 -64.66 25.61 0.07 -0.06
```

### Full example
Here is an example script that solves the first two panels of tutorial:
```
version 0
start newgame

// this tas was made with the following parameters:
// res: 1080p
// fov: 84 (default)
// fps: 60

// Solve start panel
6>P
+1>|-15 7
+1>P|0 0
+1>|50 0
+1>USP|0 0

// walk to second panel
+410>|-15 0
+15>|0 0

// solve it
+5>P
+1>|-220 -50
+1>P|0 0
+1>|-500 500
+1>P|0 0
```

## Tips & tricks

### Fast adjusting of tick values in the UI
There are many ways to change the values in the text boxes:
- Clicking the box and typing in the number (the normal way)
- Clicking the box and dragging left or right
- Hovering over the box and scrolling (shift makes it increase 10x, ctrl 100x, and both 1000x)

This last one only works for boxes that specify ticks.
