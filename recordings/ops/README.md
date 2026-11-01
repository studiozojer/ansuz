# Ansuz recording service operations

The existing Jera VPS hosts an independent **Ansuz** Snapback service at `https://ansuz.jera.ink`. Jera's endpoint, identity service, PDS and database remain separate.

| Item | Location |
| --- | --- |
| Runtime and authored schema | `/opt/ansuz-recordings` |
| Whole durable deployment, including file assets | `/var/lib/ansuz-recordings/deployment` |
| Service and OS account | `ansuz-recordings` |
| Loopback listener | `127.0.0.1:4431` |
| Caddy fragment | `/etc/caddy/ansuz-recordings.caddy` |
| Nightly backup | `ansuz-recordings-backup.timer`, 04:40 UTC plus up to five minutes |
| Backup archives, checksums, 14-day retention | `/var/backups/ansuz-recordings` |
| Restore drill | Temporary deployment on `127.0.0.1:4432` |

Node 24.11.0 and Snapback 0.4.33 are installed privately with a pinned npm lockfile and checked Node archive checksum. The service runs unprivileged, with its own writable directory and a 1 GiB memory ceiling. Caddy uses `tls force_automate` to distinguish the endpoint from the PDS wildcard policy. Only HTTPS is public; development persona headers are stripped.

Copy `../package.json`, `../package-lock.json`, `../snapback/schema.q` and this directory's scripts/unit files into the corresponding paths under `/opt/ansuz-recordings`. Do not copy development stores, credentials, node_modules or a replacement deployment directory. Before updating, run `systemctl start ansuz-recordings-backup.service`. Run `bash /opt/ansuz-recordings/install-vps.sh`; for the first proxy setup run `python3 /opt/ansuz-recordings/install-proxy.py`. The installer previews schema adoption and stops only Ansuz briefly for adoption. A failed adoption restarts its previous service. Never use destructive schema flags casually. Verify Ansuz's `/auth/config`, Jera's `/auth/config` and the PDS's `/xrpc/_health` afterward.

`backup.sh` stops only Ansuz, snapshots the entire deployment (SQLite, sidecars, releases, authentication state and assets), checksums the archive, and restarts even on an archive failure. A maintenance lock prevents overlapping deploy/backup operations. `python3 /opt/ansuz-recordings/restore-check.py` creates a synthetic probe if needed, takes a new snapshot, restores into a temporary directory, authenticates to a separate server, and checks the exact trace checksum. It never replaces live state.

Backups are private account data. Same-VPS backups do not survive loss of that VPS. An off-host snapshot has not been copied: automatic approval review requires explicit approval for exporting private account state and recordings to this Mac. That optional approval is pending; scheduled off-host replication is not installed. For recovery, preserve the damaged deployment first, verify the selected archive checksum, restore the whole directory separately, validate it with the pinned runtime, then explicitly select the recovery point. Do not restore only `store.sqlite` or omit `assets/`.
