#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CERT_DIR="$ROOT_DIR/examples/certs"

mkdir -p "$CERT_DIR"

openssl genrsa -out "$CERT_DIR/ca.key" 4096
openssl req -x509 -new -nodes -key "$CERT_DIR/ca.key" -sha256 -days 3650 \
  -subj "/CN=A2A Demo CA" \
  -out "$CERT_DIR/ca.pem"

openssl genrsa -out "$CERT_DIR/server.key" 2048
openssl req -new -key "$CERT_DIR/server.key" -subj "/CN=localhost" -out "$CERT_DIR/server.csr"
openssl x509 -req -in "$CERT_DIR/server.csr" -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca.key" \
  -CAcreateserial -out "$CERT_DIR/server.pem" -days 3650 -sha256

openssl genrsa -out "$CERT_DIR/client.key" 2048
openssl req -new -key "$CERT_DIR/client.key" -subj "/CN=A2A Demo Client" -out "$CERT_DIR/client.csr"
openssl x509 -req -in "$CERT_DIR/client.csr" -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca.key" \
  -CAcreateserial -out "$CERT_DIR/client.pem" -days 3650 -sha256

rm -f "$CERT_DIR/server.csr" "$CERT_DIR/client.csr" "$CERT_DIR/ca.srl"

echo "Generated certs under: $CERT_DIR"
