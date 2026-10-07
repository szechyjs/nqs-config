# Contributing

## Setup

See [README.md](./README.md#development) for prerequisites and dev commands
(`cargo tauri dev` / `cargo tauri build`).

## Before opening a PR

- Run the Rust tests: `cd src-tauri && cargo test`
- Run `cargo fmt` on any Rust you touched
- Run `pnpm build` to confirm the frontend typechecks and builds
- Keep PRs focused — one change per PR is easier to review than a bundle of
  unrelated fixes

## ECU protocol changes

Anything touching `kwp.rs` or `nqs.rs` (KWP2000 services, security access,
`$22`/`$24` bit layout) should cite the source for the change (e.g. the SDX
database table/field, or hardware testing notes) in the PR description —
these values come from reverse-engineering the ECU, not a public spec, so
unverifiable changes are hard to trust.

## License

This project is licensed under [AGPL-3.0-or-later](./LICENSE). By submitting
a contribution, you agree it's licensed under the same terms.
