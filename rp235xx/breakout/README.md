# Canonical-Toolchain
A template for the TERMINUS program's various RP2040/RP2350 architecture based RTIC applications

## This is a minimum example for the RP2040 on a Pi Pico, RP2350 Support is planned but not yet tested

After you have cloned that, you should be able to run our software!

Cheers!

# Debugging Info:

## Pre-requisites-

Text-Editor:
- VScode (In linux, or with wsl extension)

OS:
- Linux environment or WSL + any distro (tested with Ubuntu 24.04)

Extensions:
- "Raspberry Pi Pico" VSCODE extension

Dependencies:
- LLvm tools (rustup component add llvm-tools-preview or sudo apt install llvm-tools-preview)
- cargo-binutils ( cargo install cargo-binutils or sudo apt install cargo-binutils)
- gdb-multiarch (sudo apt install gdb-multiarch)

Project: 
- Use the canonical toolchain (includes launch.json and tasks.json files to automate debugging commands)

## Setup pt 1. Converting a rp2040 pico into a debug probe:


1. Go to https://github.com/raspberrypi/debugprobe/releases
2. Download "debugprobe_on_pico.uf2"
3. Press bootsel button on the pico, connect to your computer, and drop the "debugprobe_on_pico.uf2" into the opened directory.

## Setup pt 2. Connecting debug probe to the rp2350 pico2:

Wire according to the following image. Left is the rp2040 pico acting as a debugger, right is the rp2350 target pico 2 (The pico we will program).
![alt text](image.png)

## Setup pt 3 (If on Windows). Allow WSL to access the pico debugger.

Follow this guide up to the point of attaching your usb device to wsl:

https://learn.microsoft.com/en-us/windows/wsl/connect-usb

You will have to re-attach your device every time you connect the debug pico to your laptop with a usb cable, or debugging will not work.

# Actually Debugging

Assuming you have created a branch from the canonical toolchain and installed the pico extension you should be able to 

Build: 
1. Go to terminal
2. run build task

Flash: 
1. Go to terminal
2. run task
3. "Flash Rust Project"
(Whatever is the most recent version you have ran the build task on will be what is flashed)

Debug: 
1. Add breakpoints you wish to hit in your program file(s)
2. Go to "Run and Debug" Button on the left panel of vscode
3. Click the green play button

## Important Note

The following file names in the config files will have to be changed to match your outputted .elf file name found in the folder:
/target/thumbv8m.main-none-eabihf/debug/

(the name is "canonical-toolchain" by default if project is not renamed)

### Paths to change
In .vscode/tasks.json:

![alt text](docs/images/image-1.png)

In .vscode/launch.json:

![alt text](docs/images/image-2.png)



