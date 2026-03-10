You are helping the user create a new release of the **tracker** project (repo: `njanetos/tracker`).

## Step 1 — Choose version bump

Read the current version from `Cargo.toml`.

Ask the user:

> "Current version is **X.Y.Z**. What type of version bump?"

Offer choices: **patch**, **minor**, **major**.

Compute the new version accordingly (e.g. `0.1.2` → patch `0.1.3`, minor `0.2.0`, major `1.0.0`).

## Step 2 — Deep-dive changelog analysis

Find the previous release tag:

```bash
git tag --sort=-creatordate | head -1
```

Then perform a thorough analysis of every change since that tag:

1. Get the commit list: `git log <prev_tag>..HEAD --oneline`
2. Get the full diff: `git diff <prev_tag>..HEAD --stat` for an overview
3. For each significantly changed file or area, read the actual diffs (`git diff <prev_tag>..HEAD -- <file>`) and the current source code to understand what the change actually does from a user/developer perspective
4. Look at PR titles and descriptions if available: `gh pr list --state merged --search "merged:>$(git log -1 --format=%ci <prev_tag>)" --limit 50`

From this analysis, write a **user-facing changelog** in markdown. Use these section headers (omit any that have no entries):

- **New Features** — new user-visible capabilities
- **Improvements** — enhancements to existing functionality
- **Bug Fixes** — corrected behavior
- **Internal** — refactors, CI changes, test improvements (keep brief)

Each entry should be 1–2 sentences explaining *what changed and why it matters*, not just restating the commit message. Reference PR numbers where applicable.

## Step 3 — Confirm with user

Show the user:

> ## Release vX.Y.Z
>
> [changelog]
>
> Does this look right? Any changes before I publish?

Wait for approval. Revise if requested.

## Step 4 — Execute the release

Once approved, run these steps in order:

1. Update the `version` field in `Cargo.toml` to the new version
2. Run `cargo check` so `Cargo.lock` updates
3. Stage the changes:
   ```bash
   git add Cargo.toml Cargo.lock
   ```
4. Commit:
   ```bash
   git commit -m "chore: bump version to vX.Y.Z"
   ```
5. Tag:
   ```bash
   git tag vX.Y.Z
   ```
6. Push (this triggers the CI release build):
   ```bash
   git push origin main --follow-tags
   ```
7. Create the GitHub release with the changelog:
   ```bash
   gh release create vX.Y.Z --title "vX.Y.Z" --notes "<changelog>"
   ```

## Step 5 — Done

Return the release URL to the user. Let them know the CI will build and attach binaries for Linux, macOS, and Windows automatically.
