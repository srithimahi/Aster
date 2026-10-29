## Aster!

Aster is a terminal emulator and multiplexer that lets you have multiple panes, operations and activities all in the same application!

I created Aster mainly as a challenge to reverse-program my window terminal, but I always wanted to fix the problem of having multiple windows open to check my task manager, AI agents and my terminal, itself. Hence, Aster was born as a product of my boredom

I build this with most of my systems programming knowledge in C++ and honestly it was a whole new experience where I learnt that terminals are way harder to make than they seem.

### Features

**Terminal**

- Powershell is running through a Windows PTY
- Custom terminal grid and state
- ANSI/VT escape sequence
- Cursor handling (winit for selection cursor)
- Scrollback (in every pane including the system monitor one)
- Dynamic panes that auto-resize
- Window and pane resizing

**Multiplexer**

- Multiple tabs (that can individually be renamed)
- Horizontal and vertical pane splitting
- Recursive pane layout
- Shortcuts to select through pane (Keyboard focus navigation)
- Draggable pane resizing
- Pane zoom and unzoom
- Pane closes with auto-text wrapping back to normal

**System Monitor**

All Aster panes don't need to have terminals
So, the built-in system monitor

- CPU usage
- Memory usage
- Disk information
- Network information
- System uptime
- RUnning process list
- Process sorting (was a little complex for me)
- Process can be filtered by name or PID
- Process Inspector
- Process can also be terminated with a confirmation and
  Aster can't kill itself

**Aster Commands**

So, Aster has its own command system alongside Powershell.

If you type : into a fresh terminal prompt to open this mode, you can access these commands

- shortcuts command to see all the shortcuts
- creating and closing tabs
- splitting and closing panes
- navigating between panes
- opening the system monitor
- terminal search
- zooming and unzooming panes
- changing themes

There is also a fully executable command palette that you can use to search and execute commands in the same window, there are a lot more commands in that.
`Ctrl+Shift+P`

Oh, yes there is also command history when you click arrow up or arrow down on the terminal window.

**Themes**

Aster discovers themes from the themes/ folder

You can change themes by using Aster's command window by typing

`:`

into a fresh terminal window.

Then, type themes. And, any one of the themes below or you can click tab and you can see the autocomplete options there.

- Stardance
- Nebula
- Sunshine
- Mono

Themes can be changed while Aster is running

### How Aster Works

One of the biggest things I learnt while making Aster is that:

_A terminal is not a shell_

Powershell is the shell. Aster is the terminal emulator wrapped around it.

The pipeline looks something like this:

`Keyboard -> Aster -> PTY <-> Powershell -> raw bytes -> ANSI / VT parser -> terminal state + grid -> renderer -> window`

Powershell run inside a pseudo-terminal (PTY). Aster would read this resulting byte stream, interprets terminal control sequences, updates its own terminal state, and renders that state to the screen.

## Running Aster

**Requirements**

Aster is built for Windows and uses Windows PTY/PowerShell functionality.

You'll need Windows 10/11 and Rust

**Rust Installation**

You can install Rust at the link below.

<https://rustup.rs/>

**Clone**

`git clone https://github.com/srithimahi/Aster.git`
`cd Aster`

**Run**

`cargo run --release`

That's it!

The PT Mono font is used by Aster under assets/fonts/, along with its license.

## What I learned

Aster started as a experiment in learning Rust.

It ended up teaching me that a terminal emulator sits at the intersection of far more systems than I expected: processes, PTYs, byte streams, parsers, state machines, rendering, input handling, recursive layouts, operating-system APIs, and shell behavior.

## Font License

The license is included at:

`assets/font/OFL.txt`

## Author

Built by _Srithika_

Aster was built for Hack Club Stardance and it is definitely a long-term commitment for me and it was super fun. Thank you for reading through this!
