# Blockers

Rule for this rewrite: no crate or npm package beyond what skateboard already uses. If parity needed one, stop and write it here instead of adding it.

## Vitest (not added)

- The task brief said "Vitest for front end"
- Vitest is not a skateboard dependency, so adding it is a new npm dep: stopped here instead
- It is also on the house prohibited list ("never Vitest"), and canonical skateboard tests the front end with Node's `node --test`
- What was done instead: front-end logic lives in `src/lib/cart.ts` and `src/lib/menu.ts`, tested with `node --test` next to the source (`*.test.ts`), run by `npm test`
- Not covered without a DOM test library: React component rendering. Covered instead by a scripted browser run against the Rails copy and this app (REPORT.md)
- Steve's call: keep `node --test` (house standard) or approve Vitest (plus a DOM environment such as jsdom/happy-dom, more deps)

## Crates

- None needed. `backend/Cargo.toml` `[dependencies]` is empty; `Cargo.lock` lists only `skateboard-backend`
