# The Renyi toolchain as an MCP server in a container: the released
# binary through install.sh, serving the directory mounted at /work.
#   docker build -t renyi .
#   docker run -i -v "$PWD:/work" renyi
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
ARG RENYI_VERSION
ENV RENYI_INSTALL_DIR=/usr/local/bin
RUN curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh
WORKDIR /work
ENTRYPOINT ["renyi", "mcp", "/work"]
