FROM ubuntu:noble

# Same command so that old apt caches are not used
RUN apt-get update && \
    apt-get -y upgrade -y && \
    apt-get -y install git curl libudev-dev && \
    apt-get -y install build-essential pkg-config libusb-1.0-0-dev cmake sudo

# Delete the user with UID 1000 if it exists (fixes a problem with the devcontainer)
RUN userdel -r $(awk -F: '$3 == 1000 { print $1 }' /etc/passwd) || true

ENV user="terminus"
ENV home="/home/${user}"

RUN useradd -m -b "/home" -U ${user} \
    && usermod -a -G dialout ${user} \
    && usermod -a -G sudo ${user} \
    && chsh -s /usr/bin/bash ${user} \
    && passwd -d ${user}

USER ${user}

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="${home}/.cargo/bin:${PATH}"

# Install rust targets and binaries
RUN ${home}/.cargo/bin/rustup target add thumbv8m.main-none-eabihf
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN ${home}/.cargo/bin/cargo binstall cargo-make
RUN ${home}/.cargo/bin/cargo binstall probe-rs-tools

# Install picotool
RUN mkdir -p ${home}/picotool_install
WORKDIR ${home}/picotool_install
RUN git clone --recurse-submodules=lib/mbedtls https://github.com/raspberrypi/pico-sdk.git
RUN git clone https://github.com/raspberrypi/picotool.git
RUN cd picotool && mkdir -p build && cd build && PICO_SDK_PATH=${home}/picotool_install/pico-sdk cmake .. && sudo make install
WORKDIR /workspace

# Set the default rustup version to stable
RUN rustup default stable
# Set PATH to include /working/.cargo/bin
ENV PATH="${home}/.cargo/bin:${PATH}"
