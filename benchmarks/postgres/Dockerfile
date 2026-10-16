# Native-arch PostgreSQL + PostGIS for benchmarks
# Builds PostGIS from source to ensure native ARM64/AMD64 performance
# Inspired by vinayakkulkarni/telemetry postgres setup

ARG POSTGRES_VERSION=16

FROM postgres:${POSTGRES_VERSION}-bookworm AS postgis-builder

ARG POSTGRES_VERSION=16

USER root

RUN apt-get update && apt-get install -y --no-install-recommends \
    autoconf \
    automake \
    libtool \
    build-essential \
    ca-certificates \
    git \
    pkg-config \
    bison \
    flex \
    xsltproc \
    docbook-xsl \
    docbook-mathml \
    postgresql-server-dev-${POSTGRES_VERSION} \
    libgdal-dev \
    gdal-bin \
    libgeos-dev \
    libproj-dev \
    libjson-c-dev \
    libxml2-dev \
    libprotobuf-c-dev \
    protobuf-c-compiler \
    libpcre2-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /tmp
RUN git clone --depth 1 --branch 3.6.4 https://github.com/postgis/postgis.git && \
    cd postgis && \
    ./autogen.sh && \
    ./configure \
        --with-pgconfig=/usr/lib/postgresql/${POSTGRES_VERSION}/bin/pg_config \
        --with-raster \
        --with-topology \
        --with-gdalconfig=/usr/bin/gdal-config && \
    make -j$(nproc) && \
    make install

FROM postgres:${POSTGRES_VERSION}-bookworm

ARG POSTGRES_VERSION=16

USER root

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgdal39 \
    gdal-bin \
    libgeos-c1t64 \
    libjson-c5 \
    libproj25 \
    libprotobuf-c1 \
    libxml2 \
    libpcre2-8-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=postgis-builder /usr/lib/postgresql/${POSTGRES_VERSION}/lib/postgis*.so /usr/lib/postgresql/${POSTGRES_VERSION}/lib/
COPY --from=postgis-builder /usr/lib/postgresql/${POSTGRES_VERSION}/lib/address_standardizer*.so /usr/lib/postgresql/${POSTGRES_VERSION}/lib/
COPY --from=postgis-builder /usr/lib/postgresql/${POSTGRES_VERSION}/lib/liblwgeom*.so* /usr/lib/postgresql/${POSTGRES_VERSION}/lib/
COPY --from=postgis-builder /usr/lib/postgresql/${POSTGRES_VERSION}/lib/*topology*.so /usr/lib/postgresql/${POSTGRES_VERSION}/lib/
COPY --from=postgis-builder /usr/lib/postgresql/${POSTGRES_VERSION}/lib/*raster*.so /usr/lib/postgresql/${POSTGRES_VERSION}/lib/
COPY --from=postgis-builder /usr/share/postgresql/${POSTGRES_VERSION}/extension/postgis* /usr/share/postgresql/${POSTGRES_VERSION}/extension/
COPY --from=postgis-builder /usr/share/postgresql/${POSTGRES_VERSION}/extension/address_standardizer* /usr/share/postgresql/${POSTGRES_VERSION}/extension/

ENV GDAL_DATA=/usr/share/gdal

USER postgres

HEALTHCHECK --interval=5s --timeout=3s --retries=10 \
  CMD pg_isready -U tileserver -d tiles || exit 1
