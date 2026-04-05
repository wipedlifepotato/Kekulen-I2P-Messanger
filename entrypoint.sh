#!/bin/bash
set -e

i2pd --daemon 1 --port 4444 --http.address 0.0.0.0 --http.port 7070 --sam.address 127.0.0.1 --sam.port 7656

echo "Waiting for i2pd SAM bridge (127.0.0.1:7656)..."

MAX_RETRIES=30
COUNT=0

while ! nc -z 127.0.0.1 7656; do
  sleep 2
  COUNT=$((COUNT + 1))
  if [ $COUNT -ge $MAX_RETRIES ]; then
    echo "Error: i2pd SAM bridge failed to start after 60 seconds."
    exit 1
  fi
  echo "Still waiting for SAM bridge... ($COUNT/$MAX_RETRIES)"
done

echo "SAM bridge is UP! Starting Kekulen..."
socat TCP-LISTEN:9999,fork,reuseaddr TCP:127.0.0.1:8080 &
exec /usr/local/bin/kekulen "$@"
