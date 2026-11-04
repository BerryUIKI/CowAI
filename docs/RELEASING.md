# Release checklist

This is a maintainer procedure, not a claim that automated releases or published packages exist.

1. Confirm the agreed milestone and release scope; verify documentation against implemented behavior.
2. Create `release/<version>` from `dev`. Update the workspace version and CLI version string together; regenerate the lockfile only as needed.
3. Write release notes covering changes, compatibility, known limitations, and upgrade steps. Do not mark planned work as shipped.
4. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo build --release --workspace --locked` on the intended supported platforms. Run desktop frontend and standalone Tauri checks separately when shipping the desktop; root workspace gates exclude it. Record actual results and platform coverage.
5. Verify `cowai --help`, mock execution, license metadata, and the included license. Inspect any distribution contents for secrets and required third-party notices.
6. Review and merge the release branch into `main` and back into `dev`. Create a `v<version>` tag on the reviewed release commit and publish release notes when authorized.
7. If distributing binaries, state the platform/toolchain, include license information, and publish checksums. Publish to a registry only after reviewing crate metadata, package contents, and dependency publishability.
8. Check quick-start links and installation instructions against the actual release artifacts.

Do not silently replace a published tag or artifact. Correct release problems through a documented follow-up release or the GitFlow hotfix process. Until a support policy is established, do not promise maintenance of older versions.
