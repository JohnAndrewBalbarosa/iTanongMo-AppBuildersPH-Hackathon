#!/bin/sh
set -eu

database="/var/lib/itanongmo/itanongmo.db"
destination="/var/backups/itanongmo"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"

install -d -m 0700 "$destination"
sqlite3 "$database" ".timeout 10000" ".backup '$destination/itanongmo-$timestamp.db'"
sha256sum "$destination/itanongmo-$timestamp.db" > "$destination/itanongmo-$timestamp.db.sha256"
find "$destination" -type f -mtime +14 -delete
