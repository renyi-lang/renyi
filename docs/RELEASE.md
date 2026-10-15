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
   `renyi_vm`, `renyi`, `renyi_c` (the C API, decision AW46; it
   depends on `renyi`, so it goes last).
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
| the documentation site | `.github/workflows/pages.yml` (which builds the module for wasm32 first, decision AW33), `tools/site.py`, `docs/`, `docs/assets/play.js` |
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

## 6. After release 0.1: the platforms (2026-10-13)

Decision AW36, the owner's answers of 2026-10-13. Release 0.2 follows
stage 35 (the language server's completion, references and rename),
by section 3 above.

1. (owner) The Visual Studio Marketplace. Create the publisher
   `renyi-lang` at <https://marketplace.visualstudio.com/manage> with a
   Microsoft account, and a personal access token in Azure DevOps
   (organization "All accessible organizations", scope "Marketplace:
   Manage"). Then, from `editors/vscode/` at the release's tag:
   `npx --yes @vscode/vsce@3 publish --packagePath <the release's .vsix>
   -p <token>`, or `npx --yes @vscode/vsce@3 login renyi-lang` once and
   `publish` without the token after it. `package.json` carries the
   publisher, the icon, the license, the repository, the categories and
   the keywords the Marketplace requires; its page shows `README.md`
   and `CHANGELOG.md`. The token never enters the repository or a log.
   Done on 2026-10-13 through the upload path, without a token
   (section 7, item 5).
2. Open VSX (the registry of VSCodium, Cursor, Gitpod and others): an
   Eclipse account with the publisher agreement signed, an access
   token from <https://open-vsx.org/user-settings/tokens>; once
   `npx --yes ovsx create-namespace renyi-lang -p <token>`, then per
   release `npx --yes ovsx publish <the release's .vsix> -p <token>`.
   Done up to the publish on 2026-10-13: the owner's account and
   token exist, the namespace `renyi-lang` was created and the token
   verified against it (`ovsx verify-pat renyi-lang`); the token
   stays with the owner, outside the repository and every log, and
   reaches `ovsx` through its environment variable `OVSX_PAT`. The
   publish itself waits for release 0.2 (the owner's choice: the
   0.1.0 package is the grammar alone, without the language client
   and the icon).
3. GitHub's front, set on 2026-10-13 with `gh repo edit`: the
   description (the one line of `docs/design/08-positioning.md`
   section 6), the homepage `https://renyi-lang.org` and the topics
   `programming-language`, `ai-agents`, `agentic`, `mcp`, `mcp-server`,
   `claude-code`, `codex`, `capability-based-security`, `effect-system`,
   `reproducibility`, `sandbox`, `rust`, `webassembly`,
   `language-server`, `vscode-extension`. (owner) The social preview
   image `docs/assets/social-preview.png` (decision AV1) uploaded in the
   repository's settings: done on 2026-10-13 (section 7, item 5).
4. GitHub's recognition of `.ry` as a language is Linguist's
   (`github-linguist/linguist`), and Linguist adds a language only once
   GitHub's search finds at least 2,000 files of the extension indexed
   in the last year, forks excluded, spread across repositories and not
   mostly the language's owner's (its `CONTRIBUTING.md`, "Language
   extension and filename usage requirements"); `.ry` and `.renyi` are
   free in its table. When the bar is met, the pull request carries: an
   entry in `lib/linguist/languages.yml` (`Renyi`, type `programming`,
   a `color` (the site's accent, `LIGHT["accent"]` in `tools/site.py`),
   `extensions` `.ry` and `.renyi`, `tm_scope` `source.renyi`,
   `ace_mode` `text`, a fresh `language_id`), the grammar added with
   `script/add-grammar https://github.com/renyi-lang/renyi` (it takes
   `editors/vscode/syntaxes/renyi.tmLanguage.json`, Apache-2.0),
   samples under `samples/Renyi/` taken from `examples/`, and the
   search query of the template as the evidence. Until then GitHub
   shows a `.ry` file as plain text and counts no Renyi in the language
   bar; `.gitattributes` cannot change that (`linguist-language` names
   only a language of the table).
5. The announcements, at release 0.2: Show HN and lobste.rs, the MCP
   directories and the awesome lists (the official MCP registry,
   Smithery, a pull request to awesome-mcp-servers), the owner's X and
   LinkedIn. A session writes the texts from section 6 of the
   positioning document, with the game's page (`docs/game.md`) and the
   starter pack as the hooks and a screenshot of the game; the owner
   posts them.

## 7. Release 0.2.0, as it went (2026-10-13)

Section 3's steps ran in order in about fifteen minutes, the checks
after them in fifteen more; the lane
`D:\Projects\.worktrees\Renyi\wasm\release02\` keeps the logs and
the scripts. What the procedure did not say:

1. The bump touched the nine `version` lines of `Cargo.toml` and one
   more since 0.1.0: `crates/renyi_web/Cargo.toml` names `renyi_vm`
   with its own path and version (decision AW29, to turn the default
   features off), so it is bumped by hand with the table. Then `cargo
   update --workspace`, the fixture's toolchain line and its new hash
   in the lockfile and the two places of the packages test (section 2,
   step 4), the extension's `package.json`, its `package-lock.json`
   (the root entry, twice) and the changelog's "Unreleased" heading.
   The gates (fmt, clippy, the 64 suites and 452 tests, the lint, the
   token budget) passed at the first run; the commit is 6c999d1, and
   CI was green on it before anything was published.
2. The crates: the eight `cargo publish -p` ran in one loop that waits
   on a 429, which never came. A new version of an existing crate is
   not limited like a new crate: the eight were on crates.io in two
   minutes (00:35 to 00:37 UTC on the 14th), each verified from its
   tarball by cargo, with the login already on the machine. `cargo
   install renyi --version 0.2.0` then built the binary from crates.io
   on the owner's machine in 2 minutes 20 seconds and ran the hello
   program.
3. The tag was pushed after the last crate, as section 2 orders it.
   The Release workflow took eight minutes (00:40 to 00:48 UTC): the
   three builds in parallel with the extension, then the release. The
   page carries the three archives (5.8 to 7.4 MB), their `.sha256`
   files and `renyi-v0.2.0.vsix` (128 KB: the bundled client, the
   grammar, the icon, the readme, the changelog, the license).
   `install.sh` under WSL installed 0.2.0 from the release into the
   lane and ran `examples/hello.ry` and the tests of
   `examples/invoice.ry`.
4. Open VSX: the release's `.vsix`, downloaded from the release page
   with `gh release download`, was published with the lane's
   `ovsx_publish.py` (`ovsx verify-pat renyi-lang`, then `ovsx
   publish`; "Published renyi-lang.renyi v0.2.0"). The script reads
   the owner's token from the owner's mailbox into the environment of
   `ovsx` and masks it in its output; no token was printed or written.
   Two things to know: `ovsx` prints an emoji before every line, which
   Windows's console code page refuses, so the script decodes its
   output as UTF-8 and runs under `python -X utf8`; and the registry's
   API (`/api/renyi-lang/renyi`) still answered "not found" for some
   minutes after the publish, which is its processing, not a failure.
5. The Marketplace, later the same evening, through the owner's Chrome
   with the Claude extension (the owner's sessions and password
   manager; the session never saw or entered a credential). The owner
   signed in with the Microsoft account of `skyman.bp@gmail.com` (the
   first use of the Visual Studio services on an account asks for a
   profile and the terms; the owner clicked that page's Continue), the
   session filled the publisher form (the ID `renyi-lang`, the name
   `Renyi`, the one line of the positioning document, the icon, the
   site, the issues page, the repository; the form's inputs take typed
   text, not a programmatic value, and the ID field copies the name as
   it is typed, so the ID is set after the name), the owner approved
   the publisher agreement, and the release's `.vsix` was uploaded
   through "New extension, Visual Studio Code" on the publisher's page.
   No token was created, and none is needed on this path (section 6,
   item 1 names the token path for `vsce`); the Marketplace verifies
   the package for some minutes before the page is public. The social
   preview (section 6, item 3) was uploaded the same way in the
   repository's settings.
6. The announcements' texts (Show HN, lobste.rs, X, LinkedIn, the
   MCP directories' entry and the awesome-list line) are in the lane's
   `announcements_0.2.0.md`, with screenshots of the game's page taken
   through Playwright's headless Chromium by `game_shot.py` (the keys
   held for a few hundred milliseconds: the game reads them per frame,
   and a press shorter than a frame is lost); the owner posts them.

## 8. Release 0.2.1, as it went (2026-10-14)

A patch release the day after 0.2.0: the scope of the current directory
(`filesystem.read(".")` refused every relative path at 0.2.0), the
skill file's bullet from the first-run measure, and the toolchain listed
on the official MCP registry. Section 3 ran in order; the lane is
`D:\Projects\.worktrees\Renyi\wasm\release021\`. What was new:

1. The bump as section 7 item 1 says, with one line more:
   `readme = "../../README.md"` in `crates/renyi/Cargo.toml`. The crate
   `renyi` had no README on crates.io, and the registry verifies a
   Cargo package by a visible `mcp-name: <server name>` line in the
   crate's README as crates.io renders it (an HTML comment is
   stripped), so the repository's README carries that line under
   Install. `server.json`, the registry's manifest (validated first by
   `POST /v0.1/validate`, which holds the description to 100
   characters), and a `Dockerfile` (the released binary through
   `install.sh`, `renyi mcp /work`; what Glama's listing check starts)
   came with the commit (9d8c382). The gates: 64 suites, 454 tests, the
   lint and the token budget at the first run; CI green in six minutes.
2. The crates: the lane's `publish_crates.sh`, eight in two minutes
   (01:20 to 01:22 UTC on the 15th), no rate limit.
3. The tag after the last crate; the Release workflow eight minutes
   (01:23 to 01:31 UTC): the three archives (5.8 to 7.5 MB), their
   `.sha256` files, `renyi-v0.2.1.vsix` (128 KB). The installer's
   check came from `renyi-tools`' CI, which installs the latest release
   through `install.sh` and passed on 0.2.1 (item 7).
4. The MCP registry, by DNS authentication and without the publisher
   binary: the lane's `mcp_registry.py` (requests and cryptography)
   makes an Ed25519 key, kept in the lane, and prints the TXT record for
   the apex, `v=MCPv1; k=ed25519; p=<public key>`, which `cf.py txt`
   adds at `renyi-lang.org` (the apex, not a selector: the registry
   looks only there); the login signs an RFC 3339 timestamp and
   exchanges it at `/v0.1/auth/dns` for a registry token, held in
   memory for the one `POST /v0.1/publish`. `org.renyi-lang/renyi`
   0.2.1 was active at 01:23 UTC. The domain's namespace is
   `org.renyi-lang/*`; the GitHub route (`io.github.renyi-lang/*`)
   would have needed the owner's device-code login in a browser.
5. Open VSX with the lane's `ovsx_publish.py`: "Published
   renyi-lang.renyi v0.2.1". The namespace `renyi-lang` is verified
   since the foundation closed the claim (issue 14214) on the 14th.
6. The Marketplace's verified domain cannot be applied for before
   2027-04-13: the Marketplace requires an extension listed for six
   months and a domain registered for six months. The upload of the
   `.vsix` goes through the publisher's page as section 7 item 5 says.
7. `renyi-tools`: sitegen and bills declare `filesystem.read(".")` and
   `filesystem.write(".")` on every function with effects and on their
   tests (a function's scoped grant does not cover a callee's unscoped
   need, so the scope goes all the way down), the gate script writes
   its outputs inside each tool's `.check/` (an absolute output path is
   outside the scope), and the READMEs say so; the two test names were
   shortened to keep their heads under the line width. CI green on
   843d1f7.

## 9. Release 0.3.0, as it went (2026-10-15)

The C API (decisions AW46 to AW48: the crate `renyi_c`, the Python
binding, a library archive per target) and the fix of a Windows path
in `Sandbox::load`, the day after 0.2.1. Section 3 ran in order; the
lane is `D:\Projects\.worktrees\Renyi\wasm\release030\`. What was
new:

1. The bump as section 7 item 1 says (ten `version` lines of
   `Cargo.toml`, `renyi_web`'s line, the extension's manifest, lockfile
   and changelog, `server.json`, the fixture's toolchain line and its
   hash in the lockfile and the packages test), with the README's
   release paragraph. The gates (fmt, clippy, 67 suites, the lint, the
   token budget) passed at the first run; the commit is 303ee9a.
2. CI failed on it: the C and the Python example printed the
   toolchain's version on standard output, and the expected files CI
   compares carried "renyi 0.2.1". Both examples print the version on
   standard error since 2612957, and the expected files lose the line,
   so a bump no longer touches them. Tag after CI is green on the bump,
   as the procedure says; it took one fix this time.
3. The crates: the lane's `publish_crates.sh` with `renyi_c` ninth,
   14:41 to 14:43 UTC, no rate limit; cargo's wait for the new crate
   `renyi_c` to be visible timed out, which is the index's delay and
   not a failure (the publish had answered 200, and the crate's page
   showed 0.3.0 within the minute).
4. The tag after the last crate; the Release workflow twelve minutes
   (14:44 to 14:56 UTC): the three binary archives (5.8 to 7.4 MB),
   the `.vsix` (128 KB) and the three `renyi_c-v0.3.0-<target>`
   archives (36 to 42 MB compressed; the static library is 105 to 112
   MB unpacked, because `lto = "fat"` leaves LLVM bitcode in it; the
   shared library is 14 to 16 MB). The Windows archives' checksums
   were verified by hand and the Windows binary ran the hello program;
   the Windows C archive's `example.py` ran against the `renyi_c.dll`
   beside it, the archive's layout the binding looks in first.
5. The MCP registry: `mcp_registry.py validate` then `publish` with the
   lane's key, `org.renyi-lang/renyi` 0.3.0 active at 14:44 UTC, 0.2.1
   kept as an earlier version.
6. Open VSX with the lane's `ovsx_publish.py`: "Published
   renyi-lang.renyi v0.3.0". The Marketplace upload is the owner's
   step as before (section 7 item 5).
7. Glama's Auto-Release took the GitHub release: the server's page
   names 0.3.0 and the badge stays "rated A".

The Marketplace upload of `renyi-v0.3.0.vsix` was done by the owner
on 2026-10-15 ("[Succeeded] Extension publish on Visual Studio
Marketplace", 17:21). Open after it: the static library's size (strip
the bitcode, or a release profile for the archive without fat LTO).
Settled the same night, for release 0.4.0: the bitcode is in every
object of the archive, the standard library's included, so a profile
without LTO only brought the Linux archive from 121 to 66 MB; the
toolchain's `llvm-objcopy` (`rustup component add llvm-tools`) with the
bitcode sections removed and the debug info stripped brings it to 42.6
MB, and the C example links against the result and prints its expected
output on Linux and on Windows (the lane's `strip_c_lib.sh`; the Windows
archive from 395 to 58 MB on a debug build). The release workflow runs
that step after the C API's build, on every target.
