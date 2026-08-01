# kloc development notes

## Randomized tests: seed SOP (recorded)

Property / random tests (e.g. history-consistency vs source-tree consistency)
MUST use real randomness, not a fixed deterministic seed:

- **Different seed every run** — derive the seed from system entropy at
  startup (e.g. `SystemTime` / `OsRng`), not a hard-coded constant.
- **Report the chosen seed** — print it to the test output (`println!`), so a
  failure can be reproduced and filed.
- **Explicit seed override** — honour an environment variable (e.g.
  `KLOK_TEST_SEED`) so a reported failing seed can be replayed deterministically.
- **On failure, capture the seed + failing input** as a fixed regression test.

A randomized test with a fixed seed is not a randomized test — it is a
deterministic test wearing a disguise, and it misses the diversity that
property testing is meant to find.

This convention is mirrored from the global
`~/.config/opencode/AGENTS.md` testing rules (which live outside this repo and
cannot be edited from here).
