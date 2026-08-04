# Network Plan

## Default

Private LAN and approved WireGuard access only. Do not expose the observer/control API directly to the public Internet. Use an assigned/reserved address only after the network owner verifies it. Do not invent DNS records or firewall openings.

## Service Boundaries

- Browser clients reach the application/reverse proxy only through approved private paths.
- Prometheus reaches the bounded metrics endpoint through an approved scrape path.
- The simulation should not require outbound Internet access at runtime.
- Admin controls require authenticated private access; observer traffic cannot invoke them.

## Phase 0 Audit

Confirm VLAN/subnet placement, firewall policy, routing from WireGuard, existing proxy convention, TLS needs, Prometheus source address, and whether a kiosk requires special resolution/cache headers. Keep results out of generic code constants.
