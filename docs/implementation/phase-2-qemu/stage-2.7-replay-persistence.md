# Stage 2.7: Replay + Persistence

Goal: Persist CommitLog to disk, replay after reboot.

Tasks: Write CommitLog to VirtIO block device, implement replay on boot, verify determinism.

Test: Run system, reboot, verify state matches from replay.

Phase 2 Complete! Next: [Phase 3: Bare Metal](../phase-3-baremetal/README.md)
