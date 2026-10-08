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
4. bundles the client of the language server in `editors/vscode/`
   (`npm ci && npm run build`) and packages the extension as a `.vsix`;
5. publishes a GitHub Release with those files and generated notes.

`install.sh` and `install.ps1` download the archive of a release for the
machine they run on, verify the checksum and put the binary in place;
`cargo install renyi` builds the same binary from crates.io.

## 2. Release 0.1, in order

The owner's steps are marked (owner); the others a session does.

1. (owner) Create the GitHub organisation `renyi-lang` and transfer the
   repository to it as `renyi-lang/renyi` (decision AI4); GitHub
   redirects the old address. Register `renyi-lang.org` when wanted
   (done 2026-10-07; section 5, item 8).
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
   `Cargo.lock` agrees. The conformance fixture follows: the
   `toolchain` line of
   `tests/conformance/packages/registry/greeting/1.0.0/package.json`
   (what `renyi publish` writes, compared by a test), then the hash of
   that file's bytes in `tests/conformance/packages/project/renyi.lock.json`
   (`sha256:` and the hex digest) and the two places
   `crates/renyi/tests/packages.rs` spells it. Commit as "Release
   0.1.0".
5. (owner, or a session with the owner's say-so in that turn) Publish
   the crates to crates.io, in dependency order, each with
   `cargo publish -p <crate>`: `renyi_json`, `renyi_syntax`,
   `renyi_package`, `renyi_check`, `renyi_index`, `renyi_workspace`,
   `renyi_vm`, `renyi`.
   `cargo login` takes the token from the owner; the token never enters
   the repository or a log. Each crate's manifest carries its
   description, license and repository, and the crates name each
   other with versions through `[workspace.dependencies]`;
   `renyi_check` and `renyi` build from their tarballs because they
   embed copies of `library/std/` and `docs/cheatsheet.md` held equal
   by tests (decision AI5). `cargo package --workspace --allow-dirty`
   packages all eight and verifies the seven libraries from their
   tarballs; on the binary it stops with a cargo internal error ("no
   hash listed for renyi_index", cargo 1.94.1: the temporary registry
   of a workspace package lacks checksums for a binary's lockfile),
   which `cargo publish -p renyi` does not hit, as it verifies against
   crates.io once the libraries are there. The binary's tarball was
   built by hand on 2026-10-07 (`target/package/renyi-0.0.1/` with its
   dependencies pointed at the workspace) and runs.
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

## 5. Release 0.1.0, as it went (2026-10-07)

The steps of section 2 ran in order in one evening, 20:00 to 23:30 UTC,
and these are the things the procedure did not say:

1. The organisation and the transfer kept the repository's capitalised
   name; `gh repo rename renyi` made it the `renyi-lang/renyi` of
   decision AI4 (GitHub redirects the old names). The local clone's
   remote was pointed at the new address.
2. The organisation is on GitHub's Team plan, so the Pages site could be
   registered while the repository was still private:
   `gh api -X POST repos/renyi-lang/renyi/pages -f build_type=workflow`.
   The Site workflow deployed on the first push after the repository
   went public (`gh repo edit --visibility public
   --accept-visibility-change-consequences`), and the pages answered
   within a minute.
3. The version bump reaches the conformance fixture (step 4 above says
   how); the first gate run after the bump failed on it.
4. crates.io limits new crates: five published at once, then one more
   every ten minutes (`429 Too Many Requests` with the time to retry).
   Seven crates took about twenty-five minutes, with a loop that waits
   for the time the error names. Publish the libraries a day earlier,
   or budget the wait. The login was already on the machine
   (`~/.cargo/credentials.toml`); no token was read or printed.
5. The tag was pushed while the last crate waited for the limit, so
   for about an hour the release page existed and `cargo install
   renyi` did not work. Next time, tag after the last crate is on
   crates.io, as section 2 orders it.
6. The release workflow took about fifteen minutes; the Windows build
   was the slowest. The release page carries the three archives, their
   `.sha256` files and the `.vsix`.
7. Checked afterwards: `install.sh` under WSL installed and ran 0.1.0
   from the release; the Windows archive's checksum and binary were
   verified by hand (the PowerShell installer edits the user's PATH, so
   it was not run on the owner's machine); `cargo install renyi` built
   the binary from crates.io.
8. The domain (AI4), later the same evening: the owner registered
   `renyi-lang.org` on Cloudflare and handed over a credential that
   mints tokens; the session minted a token with DNS write and zone
   read for one hour, wrote the GitHub Pages records (four `A` and
   four `AAAA` on the apex, `www` a `CNAME` to `renyi-lang.github.io`,
   none proxied, so that GitHub issues the certificate), deleted the
   token, set the domain on the Pages site (`gh api -X PUT
   repos/renyi-lang/renyi/pages -f cname=renyi-lang.org`; a workflow
   deployment needs no `CNAME` file) and, once the certificate's
   state was `approved` (within minutes), enforced HTTPS (`-F
   https_enforced=true`). The old address redirects. The credential
   stays on the owner's machine, outside the repository; no token
   value was printed or written anywhere.
