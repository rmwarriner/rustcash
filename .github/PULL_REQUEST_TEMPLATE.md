## Summary

<!-- 3-5 bullets: what changed and why -->

-
-
-

## Test plan

<!-- Commands you ran locally. Must be fully reproducible — paste-and-run, no guessing. -->

```bash

```

**Expected output / behaviour:**

<!-- What a reviewer should see when they run the above -->

**CI:** [ ] All checks green on this PR

## Checklist

- [ ] `just ci` passes locally (fmt-check, clippy, tests, coverage, audit)
- [ ] New behaviour has tests (unit, property, or integration as appropriate)
- [ ] New accounting invariants have property tests (`proptest`)
- [ ] No floats used for money
- [ ] No `unsafe` blocks
- [ ] Error messages are actionable (say what failed, why, and what to do)
- [ ] ADR added / updated if this is a significant design decision

<!-- Closes #N -->
