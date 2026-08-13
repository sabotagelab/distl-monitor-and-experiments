FROM rust:1.97.1-trixie AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    fish \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# RUN pip install --upgrade pip ipython

WORKDIR /usr/src/app
COPY ./distl_experiments ./experiments
COPY ./distl ./distl

WORKDIR /usr/src/app/distl
RUN cargo build --release
WORKDIR /usr/src/app/experiments
RUN cargo build --release
