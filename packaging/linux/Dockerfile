FROM rust:1.100.0-bookworm

# Debian 12's glibc 2.36 is the binary baseline (also usable on Ubuntu 24.04+).
# Test tools stay in this image, not in the installed application.
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang cmake pkg-config libfontconfig1-dev libfreetype6-dev \
    libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libxi-dev \
    libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxcb-xinput-dev libxcb-randr0-dev libxcb-xinerama0-dev libxcb-cursor-dev \
    libasound2-dev libssl-dev libvulkan1 mesa-vulkan-drivers \
    dbus-x11 xvfb openbox xdotool x11-utils fonts-noto-core fonts-noto-cjk \
    dpkg-dev desktop-file-utils python3 python3-dbus python3-gi

ENV CARGO_TARGET_DIR=/work/target/linux-bookworm
ENV CARGO_BUILD_JOBS=4
WORKDIR /work
RUN apt-get update && apt-get install -y --no-install-recommends scrot xauth strace gdb
RUN rustup component add --toolchain 1.100.0 rustfmt clippy
