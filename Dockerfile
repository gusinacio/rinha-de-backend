FROM rust:1.76.0 as build

WORKDIR /app

RUN --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=bind,source=src,destination=src \
    --mount=type=bind,source=.sqlx,destination=.sqlx \
    --mount=type=bind,source=Cargo.toml,destination=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,destination=Cargo.lock \
    cargo build --release &&\
    cp target/release/rinha-de-backend /opt/server

WORKDIR /opt/

ENTRYPOINT [ "rinha-de-backend" ]

FROM debian:bookworm-slim AS final

RUN apt-get update && apt-get install -y libpq-dev  && rm -rf /var/lib/apt/lists/*

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#user
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

# Copy the executable from the "build" stage.
COPY --from=build /opt /bin/

# Expose the port that the application listens on.
ENV PORT=3000
EXPOSE 3000

# What the container should run when it is started.
CMD ["/bin/server"]