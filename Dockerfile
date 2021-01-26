FROM debian:stable

ARG TARGETARCH
ARG TARGETPLATFORM
ARG TARGETVARIANT
ARG BINARY=binary/${TARGETARCH}${TARGETVARIANT}/kappa

RUN apt-get update && apt-get install -y busybox-static

RUN mkdir -p  /opt/kentik/kappa/
ADD $BINARY   /opt/kentik/kappa/
RUN chmod a+x /opt/kentik/kappa/kappa

# --

FROM scratch

COPY --from=0 /bin/busybox    /bin/
COPY --from=0 /etc/group      /etc/
COPY --from=0 /etc/passwd     /etc/
COPY --from=0 /opt/kentik     /opt/kentik

WORKDIR /opt/kentik/kappa

ENTRYPOINT ["/opt/kentik/kappa/kappa"]
