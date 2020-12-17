FROM debian:stable

ARG TARGET=x86_64-unknown-linux-musl

RUN apt-get update && apt-get install -y build-essential curl

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain none
ENV PATH="/root/.cargo/bin:$PATH"

RUN rustup toolchain install stable
RUN rustup target install --toolchain stable $TARGET

RUN apt-get install -y busybox-static capnproto musl-tools

WORKDIR /work
COPY .  /work/

RUN cargo build --release --target $TARGET

RUN mkdir -p /opt/kentik/kappa
RUN cp target/$TARGET/release/kappa /opt/kentik/kappa/

# --

FROM scratch

COPY --from=0 /bin/busybox    /bin/
COPY --from=0 /etc/group      /etc/
COPY --from=0 /etc/passwd     /etc/
COPY --from=0 /opt/kentik     /opt/kentik

WORKDIR /opt/kentik/kappa
