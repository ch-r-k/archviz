# =========================
# RP2040 / Embassy Devcontainer
# =========================
FROM rust:latest

# -------------------------
# Install system dependencies
# -------------------------
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    sudo \
    libudev-dev \
    cmake \
    unzip \
    gcc-arm-none-eabi \
    openocd \
    && rm -rf /var/lib/apt/lists/*

# -------------------------
# Create vscode user and give sudo rights
# -------------------------
ARG USERNAME=vscode
RUN useradd -m -s /bin/bash $USERNAME \
    && echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# -------------------------
# Set up user-local cargo to avoid permissions issues
# -------------------------
ENV CARGO_HOME=/home/vscode/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH
RUN mkdir -p $CARGO_HOME \
    && chown -R $USERNAME:$USERNAME $CARGO_HOME

# -------------------------
# Switch to vscode user
# -------------------------
USER $USERNAME
WORKDIR /home/$USERNAME/app

# -------------------------
# Install Rust embedded target for RP2040
# -------------------------
RUN rustup target add thumbv6m-none-eabi

# -------------------------
# Install embedded flashing tools
# -------------------------
RUN cargo install --locked probe-rs-tools elf2uf2-rs

# -------------------------
# Install rust-analyzer CLI for VS Code
# -------------------------
RUN curl -L https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-linux -o $CARGO_HOME/bin/rust-analyzer \
    && chmod +x $CARGO_HOME/bin/rust-analyzer