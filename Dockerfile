FROM rust:1-buster as build

# create dummy application for dependency caching
RUN USER=root cargo new --bin throttled
WORKDIR /throttled

# download + compile dependencies for caching
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN rm src/*.rs

# build for real
COPY src ./src
RUN rm ./target/release/deps/throttled*
RUN cargo build --release

RUN wc -c target/release/throttled | numfmt --to=iec-i

# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------

# FROM amd64/busybox:uclibc as busybox
# FROM gcr.io/distroless/cc:debug
# COPY --from=busybox /bin/busybox /busybox/busybox
# RUN ["/busybox/busybox", "--install", "/bin"]
FROM gcr.io/distroless/cc:latest

COPY --from=build /throttled/target/release/throttled .

CMD ["/usr/local/bin/throttled"]
