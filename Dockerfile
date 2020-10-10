# syntax=docker/dockerfile:experimental
FROM rust:latest
ENV HOME=/home/root
WORKDIR $HOME/app
ADD pasta6_core/src pasta6_core/src
ADD pasta6_home/src pasta6_home/src
ADD pasta6_meta/src pasta6_meta/src
ADD pasta6_paste/src pasta6_paste/src
ADD pasta6_util/src pasta6_util/src
ADD pasta6_core/templates pasta6_core/templates
ADD pasta6_home/templates pasta6_home/templates
ADD pasta6_meta/templates pasta6_meta/templates
ADD pasta6_paste/templates pasta6_paste/templates
ADD Cargo.lock .
ADD pasta6_core/Cargo.toml pasta6_core
ADD pasta6_home/Cargo.toml pasta6_home
ADD pasta6_meta/Cargo.toml pasta6_meta
ADD pasta6_paste/Cargo.toml pasta6_paste
ADD pasta6_util/Cargo.toml pasta6_util
ADD Cargo.toml .
ADD build.rs .
# TODO: we don't need the git dir to compile the app, we just need it to get the commit ID, perhaps this could be an env var?
ADD .git .git
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/home/root/app/target \
    cargo build --release && \
    cp /home/root/app/target/release/pasta6_home /pasta6_home && \
    cp /home/root/app/target/release/pasta6_meta /pasta6_meta && \
    cp /home/root/app/target/release/pasta6_paste /pasta6_paste && \
    cp /home/root/app/target/release/pasta6-generate-key /pasta6-generate-key
