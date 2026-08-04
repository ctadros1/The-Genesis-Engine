# Deployment Plan

## Proposed Sequence

1. Complete read-only Phase 0 audit and approve the VM specification.
2. Create an isolated VM using the approved host/storage/network values.
3. Harden the guest: updates, non-login service account, time sync, firewall according to approved private access policy.
4. Install the pinned application/runtime and start a non-production smoke world.
5. Verify health, metrics, observer access, save/restore, and backpressure behavior.
6. Import/create the first named world only after backup and rollback checks pass.

## Rollback

Stop the application service, preserve logs/saves, restore the last validated application snapshot or VM backup in an isolated test target first, and only then return service. Do not delete data volumes as a recovery method. Record the incident and whether the live world advanced after the last durable checkpoint.

## Change Control

Each rollout lists owner, approved window, image/version/config hash, expected metrics, backup ID, rollback command/procedure, and verification result. This plan is not a command to deploy.
