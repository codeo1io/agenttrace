# random-bytes (rm-596)

4096 bytes of `/dev/urandom` in the auto-discovered opencode database
slot, from assess probes P12/P14 of run ff0068ca. Pre-fix this
rendered as "No session files found in any auto-discovered agent home"
while `--doctor` printed `OpenCode (DB) found parsed=0 failed=0` and
recommended `--demo` — an unreadable database masquerading as an
absent one. Any random content exercises the same code path; the
committed bytes pin the exact case.

- opencode.db sha256: efea7142d56415d929c26491791c629b1ed277203d4e3e8d2f9127ad87702132
