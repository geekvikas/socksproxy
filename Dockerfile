FROM rust:1.78 AS builder
COPY . .
RUN cargo build --release


FROM rust:1.78
COPY --from=builder ./target/release/socksproxy /usr/bin/socksproxy
COPY --from=builder ./start.sh /usr/bin/start.sh
RUN chmod +x /usr/bin/start.sh && mkdir -p /etc/socksproxy/
CMD "/usr/bin/start.sh"
