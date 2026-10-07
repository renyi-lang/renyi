# Releasing Renyi

Decisions AI1 to AI4 (2026-10-07). This is the procedure for release 0.1
and for every release after it: what the repository automates and what
the owner does by hand. The commands run from the repository root.

## 1. What a release is

A git tag `v<version>` on `main`, whose version is the workspace version
in `Cargo.toml`. The tag starts `.github/workflows/release.yml`, which:

1. builds the `renyi` binary with `cargo build --release --locked` on
   Linux (x86_64), macOS (Apple silicon) and Windows (x86_64), and
   refuses a tag whose version is not the workspace version;
2. runs the conformance suite (`tools/conformance.py`) against each
   release binary;
3. packages each binary with `README.md` and `LICENSE` as
   `renyi-v<version>-<target>.tar.gz` (`.zip` on Windows), with a
   `.sha256` file next to it;
4. packages the VS Code extension of `editors/vscode/` as a `.vsix`;
5. publishes a GitHub Release with those files and generated notes.

`install.sh` and `install.ps1` download the archive of a release for the
machine they run on, verify the checksum and put the binary in place;
`cargo install renyi` builds the same binary from crates.io.

## 2. Release 0.1, in order

The owner's steps are marked (owner); the others a session does.

1. (owner) Create the GitHub organisation `renyi-lang` and transfer the
   repository to it as `renyi-lang/renyi` (decision AI4); GitHub
   redirects the old address. Register `renyi-lang.org` when wanted.
   Every address in the repository already names `renyi-lang/renyi`:
   `Cargo.toml` (`repository`), `README.md`, `install.sh`,
   `install.ps1`, `editors/vscode/package.json`.
2. (owner) Make the repository public (decision V1 kept it private
   until this release).
3. (owner) In the repository's settings, Pages: source "GitHub
   Actions". The site workflow publishes `docs/` as the documentation
   site on every push to `main` of a public repository.
4. Bump the version: `version = "0.1.0"` in `[workspace.package]` of
   `Cargo.toml` and in every entry of its `[workspace.dependencies]`
   (the crates name each other through that table, with the version
   crates.io requires), then `cargo update --workspace` so that
   `Cargo.lock` agrees; commit as "Release 0.1.0".
5. (owner, or a session with the owner's say-so in that turn) Publish
   the crates to crates.io, in dependency order, each with
   `cargo publish -p <crate>`: `renyi_json`, `renyi_syntax`,
   `renyi_package`, `renyi_check`, `renyi_index`, `renyi_vm`, `renyi`.
   `cargo login` takes the token from the owner; the token never enters
   the repository or a log. Each crate's manifest carries its
   description, license and repository; `cargo package -p <crate>
   --no-verify` shows what would be uploaded.
6. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`. Watch the
   Release workflow; when it finishes, the release page lists the three
   archives, their checksums and the `.vsix`.
7. Verify on a machine that has nothing: the two one-line installers of
   `README.md` and `cargo install renyi`; then `renyi run
   examples/hello.ry Renyi` and `renyi test examples/invoice.ry`.
8. (owner) Publish the VS Code extension: a publisher `renyi-lang` on
   the Visual Studio Marketplace, then `npx @vscode/vsce publish` from
   `editors/vscode/` with its token, or upload the `.vsix` of the
   release by hand. Until then the `.vsix` installs through "Extensions:
   Install from VSIX".
9. Announce with the texts of `docs/design/08-positioning.md` section 6
   (the one line, the three sentences, the paragraph), pointing at the
   starter pack for agents (`starter/`, decision AI3) and the
   documentation site.

## 3. Every release after it

1. Bump the workspace version and `Cargo.lock`, commit.
2. `cargo publish` the crates whose sources changed, in the order
   above (a crate whose dependencies' versions changed must be published
   again too).
3. Tag `v<version>` and push the tag; the workflow does the rest.
4. Check the release page and one installer.

The version follows the semantic rule of decision G1 as `renyi index
--diff` reports it: a public definition removed, narrowed or renamed is
a major bump, one added or widened a minor bump, anything else a patch.

## 4. Where the pieces are

| Piece | Path |
|-------|------|
| the release workflow | `.github/workflows/release.yml` |
| the installers | `install.sh`, `install.ps1` |
| the VS Code extension | `editors/vscode/` |
| the documentation site | `.github/workflows/pages.yml`, `tools/site.py`, `docs/` |
| the starter pack for agents | `starter/` |
| the wording of announcements | `docs/design/08-positioning.md`, section 6 |
