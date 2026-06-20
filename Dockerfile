# Immagine del server REST di ArchMind (uso team/Enterprise).
# Build multi-stage: compila solo `archmind-server` (+ core), NON l'app Tauri
# (che richiederebbe webkit). Runtime puro: solo il binario, nessuna libreria
# di sistema (sqlite/tree-sitter/tantivy sono compilati staticamente, TLS rustls).
FROM rust:1-bookworm AS build
WORKDIR /app
COPY . .
RUN cargo build --release -p archmind-server

FROM debian:bookworm-slim
RUN useradd -m archmind && mkdir /data && chown archmind /data
COPY --from=build /app/target/release/archmind-server /usr/local/bin/archmind-server
USER archmind
EXPOSE 7878
ENV ARCHMIND_ADDR=0.0.0.0:7878 \
    ARCHMIND_DB=/data/archmind-server.db
VOLUME ["/data", "/projects"]
ENTRYPOINT ["archmind-server"]
