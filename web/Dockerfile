FROM rust:latest

WORKDIR /usr/src/envirometer
COPY . .

RUN cargo install --path .

CMD ["web"]
