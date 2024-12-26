
FROM debian:bookworm-slim

RUN mkdir -p /dexter
WORKDIR /dexter

ARG TARGETARCH

COPY output/linux/${TARGETARCH}/rpc-proxy /dexter/rpc-proxy

EXPOSE 8899

ENV RUST_LOG=info

CMD ["./rpc-proxy"]
