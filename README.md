# AMALTHEA
Software systems, tools, and libraries for the 2025 AMALTHEA Rocksat-X mission by the University of Alabama in Huntsville. The AMALTHEA mission consists of several components, all of which have crates or binaries under this repository:

1. The ICARUS payload
2. The ejector assembly
3. The relay deployable from ICARUS

A shared library for communication packets is also present, as well as a common library for shared components.

# Building and running
If `picotool` is installed, `cargo make flash_debug_usb` or `cargo make flash_release_usb` will run the compiled source on a connected microcontroller. If a `probe-rs`-compatible device is connected and hooked up to a device, `cargo make run_debug_probers` will flash the flight software to the device through the SWD interface.

Since not all binaries support testing, `cargo make test` excludes non-compatible crates from unit tests.
