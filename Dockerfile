FROM rust:1.67.0-slim-bullseye as base
RUN apt update -y
RUN apt install -y --no-install-recommends \
    ca-certificates \
    gcc \
    libc6-dev \
    libpq-dev \
    libasn1-8-heimdal \
    libtasn1-6 \
    libhdb9-heimdal \
    nettle-dev \
    libhogweed6

RUN apt clean -y

ENV USER=drasil
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

FROM base as drasil-builder
WORKDIR /
ENV CARGO_HOME=/.cargo                      
COPY ./.cargo/config $CARGO_HOME/config
COPY ./.cargo/git $CARGO_HOME/git
COPY ./.cargo/workspace /
RUN cargo build --release --target x86_64-unknown-linux-gnu


# Vidar Image
FROM gcr.io/distroless/cc as vidar
WORKDIR /vidar
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/vidar /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries to keep image small
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
EXPOSE 4101
USER drasil:drasil
CMD ["vidar"]
LABEL binary=vidar

# Heimdallr Image
FROM gcr.io/distroless/cc as heimdallr
WORKDIR /heimdallr
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/heimdallr /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries to keep image small
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
EXPOSE 4000
USER drasil:drasil
CMD ["heimdallr"]
LABEL binary=heimdallr

# Build Odin
FROM gcr.io/distroless/cc as odin
WORKDIR /odin
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/odin /usr/bin
COPY --from=drasil/builder:latest /protocol_parameters.json /odin/protocol_parameters_babbage.json
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
EXPOSE 6142
USER drasil:drasil
CMD ["odin"]
LABEL binary=odin

# Build Loki
FROM gcr.io/distroless/cc as loki
WORKDIR /loki
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/loki /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
EXPOSE 4000
USER drasil:drasil
CMD ["loki"]
LABEL binary=loki

# Build Frigg
FROM gcr.io/distroless/cc as frigg
WORKDIR /frigg
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/frigg /usr/bin
COPY --from=drasil/builder:latest /protocol_parameters.json /odin/protocol_parameters_babbage.json
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
EXPOSE 4000
USER drasil:drasil
CMD ["frigg"]
LABEL binary=frigg

# Build Geri
FROM gcr.io/distroless/cc as geri
WORKDIR /geri
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/geri /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
ENV CLUSTER=false
ENV REDIS_CLUSTER=false
ENV REDIS_DB_URL_UTXOMIND="redis://default:@127.0.0.1:6379/0"
ENV REDIS_DB_URL_TXMIND="redis://default:@127.0.0.1:6379/0"
ENV STREAM_TRIMMER=true
ENV STREAMS="transaction|block"
ENV TIMEOUT=20000
USER drasil:drasil
CMD ["geri"]
LABEL binary=geri

# Build Drasil Job Processor
FROM gcr.io/distroless/cc as drasil_jobs
WORKDIR /drasil_jobs
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/drasil_jobs /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
ENV AMQP_ADDR="amqp://rmq:rmq@127.0.0.1:5672"
ENV QUEUE_NAME="drasil_jobs"
ENV CONSUMER_NAME="drasil_jobs_default"
USER drasil:drasil
CMD ["drasil_jobs"]
LABEL binary=drasil_jobs

# Build Loki Worker
FROM gcr.io/distroless/cc as work_loki
WORKDIR /work_loki
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/work_loki /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
ENV AMQP_ADDR="amqp://rmq:rmq@127.0.0.1:5672/%2f"
ENV QUEUE_NAME="mint_response"
ENV CONSUMER_NAME="worker_loki_0"
USER drasil:drasil
CMD ["work_loki"]
LABEL binary=work_loki

# Build Freki
FROM gcr.io/distroless/cc as freki
WORKDIR /freki
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/freki /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
ENV REWARD_DB_URL=x
ENV DBSYNC_DB_URL=x
ENV PLATFORM_DB_URL=x
USER drasil:drasil
CMD ["freki"]
LABEL binary=freki

# Build Utxopti
FROM gcr.io/distroless/cc as utxopti
WORKDIR /utxopti
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/utxopti /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
# copy just the needed libraries
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/libpq.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2 /usr/lib/x86_64-linux-gnu/libgssapi_krb5.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5.so.3 /usr/lib/x86_64-linux-gnu/libkrb5.so.3
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libk5crypto.so.3 /usr/lib/x86_64-linux-gnu/libk5crypto.so.3
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcom_err.so.* /lib/x86_64-linux-gnu/
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libkrb5support.so.0 /usr/lib/x86_64-linux-gnu/libkrb5support.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2 /usr/lib/x86_64-linux-gnu/liblber-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsasl2.so.2 /usr/lib/x86_64-linux-gnu/libsasl2.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgnutls.so.30 /usr/lib/x86_64-linux-gnu/libgnutls.so.30
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libkeyutils.so.1 /lib/x86_64-linux-gnu/libkeyutils.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2 /usr/lib/x86_64-linux-gnu/libldap_r-2.4.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libssl.so.1.1 /usr/lib/x86_64-linux-gnu/libssl.so.1.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libresolv.so.2 /lib/x86_64-linux-gnu/libresolv.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libasn1.so.8 /usr/lib/x86_64-linux-gnu/libasn1.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhcrypto.so.4 /usr/lib/x86_64-linux-gnu/libhcrypto.so.4
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libroken.so.18 /usr/lib/x86_64-linux-gnu/libroken.so.18
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libp11-kit.so.0 /usr/lib/x86_64-linux-gnu/libp11-kit.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libidn2.so.0 /usr/lib/x86_64-linux-gnu/libidn2.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libunistring.so.2 /usr/lib/x86_64-linux-gnu/libunistring.so.2
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libtasn1.so.6 /usr/lib/x86_64-linux-gnu/libtasn1.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libnettle.so.8 /usr/lib/x86_64-linux-gnu/libnettle.so.8
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhogweed.so.6 /usr/lib/x86_64-linux-gnu/libhogweed.so.6
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libgmp.so.10 /usr/lib/x86_64-linux-gnu/libgmp.so.10
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libwind.so.0 /usr/lib/x86_64-linux-gnu/libwind.so.0
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libheimbase.so.1 /usr/lib/x86_64-linux-gnu/libheimbase.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libhx509.so.5 /usr/lib/x86_64-linux-gnu/libhx509.so.5
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libsqlite3.so.0 /usr/lib/x86_64-linux-gnu/libsqlite3.so.0
COPY --from=drasil/builder:latest /lib/x86_64-linux-gnu/libcrypt.so.1 /lib/x86_64-linux-gnu/libcrypt.so.1
COPY --from=drasil/builder:latest /usr/lib/x86_64-linux-gnu/libffi.so.7 /usr/lib/x86_64-linux-gnu/libffi.so.7
ENV REWARD_DB_URL=x 
ENV DBSYNC_DB_URL=x 
ENV PLATFORM_DB_URL=x 
ENV RUST_LOG=info
ENV JWT_PUB_KEY=x
USER drasil:drasil
CMD ["utxopti"]
LABEL binary=utxopti

#Build Utxopti
FROM gcr.io/distroless/cc as dvltath
WORKDIR /dvltath
COPY --from=drasil/builder:latest /target/x86_64-unknown-linux-gnu/release/dvltath /usr/bin
COPY --from=drasil/builder:latest /etc/passwd /etc/passwd
COPY --from=drasil/builder:latest /etc/group /etc/group
USER drasil:drasil
ENV RUST_LOG=info
CMD ["dvltath"]
LABEL binary=dvltath