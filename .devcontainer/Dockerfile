FROM ubuntu:noble

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install git curl libudev-dev -y

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh && \
    chmod +x rustup.sh && \
    ./rustup.sh -y && \
    rm rustup.sh
ENV PATH="/root/.cargo/bin:${PATH}"

# Install rust targets and binaries
RUN /root/.cargo/bin/rustup target add thumbv8m.main-none-eabihf
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN /root/.cargo/bin/cargo binstall cargo-make
RUN /root/.cargo/bin/cargo binstall probe-rs-tools

# Install picotool
RUN mkdir /picotool_install
WORKDIR /picotool_install
RUN git clone https://github.com/raspberrypi/pico-sdk.git
RUN git clone https://github.com/raspberrypi/picotool.git
RUN cd pico-sdk && git submodule update --init lib/mbedtls && cd ../picotool
RUN apt install build-essential pkg-config libusb-1.0-0-dev cmake -y
RUN cd /picotool_install/picotool && mkdir build && cd build && PICO_SDK_PATH=/picotool_install/pico-sdk cmake .. && make install


# Move cargo binaries to PATH
RUN mv /root/.cargo/bin/* /usr/local/bin
RUN rustup default stable
RUN mkdir /working
# Move $CARGO_HOME to /working and $RUSTUP_HOME to /working
RUN mv /root/.cargo /working
RUN mv /root/.rustup /working
# Set $CARGO_HOME and $RUSTUP_HOME to /working
ENV CARGO_HOME=/working/.cargo
ENV RUSTUP_HOME=/working/.rustup
# Set PATH to include /working/.cargo/bin
ENV PATH="/working/.cargo/bin:${PATH}"

WORKDIR /working