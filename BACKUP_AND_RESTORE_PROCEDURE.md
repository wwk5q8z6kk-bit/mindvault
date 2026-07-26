# Repository and Data Backup/Restore Procedure

This procedure is designed to preserve committed history, ignored files, local-only configuration,
runtime data, and uncommitted work. Run it from a trusted terminal while MindVault and all writers
are stopped. Never paste credentials into command arguments.

## 1. Choose a protected destination

Use an encrypted external volume with enough free space. Set a task-specific variable:

```bash
export MINDVAULT_BACKUP_ROOT="/Volumes/EncryptedBackup/mindvault-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$MINDVAULT_BACKUP_ROOT"
chmod 700 "$MINDVAULT_BACKUP_ROOT"
```

Do not target `/`, `$HOME`, `~`, or the source repository.

## 2. Quiesce

1. Stop the Rust server, Python AI service, Tauri app, frontend dev server, file watcher, sync, and
   connector processes.
2. Confirm no process has the runtime SQLite databases open.
3. Record the stop time and operator in `MANIFEST.txt`.

If quiescing is impossible, use SQLite's online backup API for each database; a raw copy of a live
WAL database is not sufficient unless the database, WAL, and SHM are captured coherently.

## 3. Capture Git identity and uncommitted work

Set the source checkout explicitly, then run from it:

```bash
export MINDVAULT_CHECKOUT="/absolute/path/to/mindvault"
cd "$MINDVAULT_CHECKOUT"
git rev-parse HEAD > "$MINDVAULT_BACKUP_ROOT/HEAD.txt"
git status --porcelain=v2 --branch > "$MINDVAULT_BACKUP_ROOT/status.txt"
git remote -v > "$MINDVAULT_BACKUP_ROOT/remotes.txt"
git tag --list --format='%(refname:short) %(objectname)' > "$MINDVAULT_BACKUP_ROOT/tags.txt"
git submodule status --recursive > "$MINDVAULT_BACKUP_ROOT/submodules.txt"
git diff --binary > "$MINDVAULT_BACKUP_ROOT/unstaged.patch"
git diff --cached --binary > "$MINDVAULT_BACKUP_ROOT/staged.patch"
git ls-files --others --exclude-standard -z > "$MINDVAULT_BACKUP_ROOT/untracked.zlist"
git bundle create "$MINDVAULT_BACKUP_ROOT/repository.bundle" --all
```

Create a filesystem archive of the entire checkout so ignored and untracked content is retained:

```bash
ditto --rsrc --extattr --acl \
  "$MINDVAULT_CHECKOUT" \
  "$MINDVAULT_BACKUP_ROOT/worktree"
```

The filesystem copy is the authoritative protection for untracked files. The patches and bundle
are independent recovery aids.

## 4. Capture runtime and human source data

Copy each existing location separately:

```bash
test ! -e "$HOME/.mindvault" || ditto --rsrc --extattr --acl \
  "$HOME/.mindvault" "$MINDVAULT_BACKUP_ROOT/runtime-dot-mindvault"
```

Also copy every configured Obsidian vault, filesystem import root, external attachment directory,
custom model directory, connector config, and Tauri/browser profile. Record the original path,
classification, owner, and whether it is canonical in `MANIFEST.txt`.

For SQLite databases, also create logical consistency evidence:

```bash
sqlite3 "$HOME/.mindvault/data/mindvault.sqlite" "PRAGMA quick_check;" \
  > "$MINDVAULT_BACKUP_ROOT/mindvault-quick-check.txt"
sqlite3 "$HOME/.mindvault/data/keychain.sqlite" "PRAGMA quick_check;" \
  > "$MINDVAULT_BACKUP_ROOT/keychain-quick-check.txt"
```

Only run these commands for databases that exist.

## 5. Hash and seal the backup

```bash
(
  cd "$MINDVAULT_BACKUP_ROOT"
  find . -type f ! -name SHA256SUMS -print0 |
    LC_ALL=C sort -z |
    xargs -0 shasum -a 256 > SHA256SUMS
)
chmod -R go-rwx "$MINDVAULT_BACKUP_ROOT"
```

Create a second encrypted copy on a different physical device. Never treat a single backup as
verified.

## 6. Restore rehearsal

Restore into a new, empty test directory and a disposable user-data root:

1. Verify `shasum -a 256 -c SHA256SUMS`.
2. Clone `repository.bundle`, check out the recorded branch/commit, then compare it to `worktree`.
3. Restore the runtime tree to a temporary location, never over the live directory.
4. Run `PRAGMA integrity_check` and `PRAGMA foreign_key_check` on both databases.
5. Count nodes, relationships, proposals, audit records, attachments, and source documents.
6. Rebuild FTS/vector/graph-derived structures from canonical sources.
7. Run exact, FTS, vector, graph, and citation spot checks using known fixtures.
8. Verify encrypted/sealed data can be unlocked with the separately held credentials.
9. Record elapsed time and differences.

## 7. Production restore gate

Do not overwrite live data until:

- two backup copies verify;
- the restore rehearsal passes;
- the target paths are explicit;
- the current live state has a new pre-restore backup;
- the operator approves the manifest and rollback point.

The current Rust database-only encrypted backup is not a complete vault backup: it reads a live
SQLite file directly and omits WAL state, blobs, keychain data, configuration, and human source
files. The CLI data-directory archive is broader but still lacks a coordinated writer pause,
integrity manifest, and transactional restore. Neither should be the sole recovery mechanism.
