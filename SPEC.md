# Homie Specification

A Rust CLI tool for managing dotfiles symlinks across multiple repositories with configurable linking strategies and template support.

## Overview

Homie orchestrates symlinks from multiple dotfile repositories to a target directory (typically `~`). It supports different linking strategies per path, preserves external symlinks, and renders templates with variable substitution.

Repos are auto-discovered at `~/.homie/repos/` and are self-contained with their own configuration.

## Core Concepts

### Repositories

Repos live at `~/.homie/repos/<name>/` and use a flat structure - the repo root directly contains the files to be linked:

```
~/.homie/repos/
├── dotfiles/              # Repo root IS the source
│   ├── homie.toml         # Repo config with target
│   ├── .zshrc
│   ├── .config/
│   │   └── nvim/
│   │       └── init.lua
│   └── .gitconfig.tmpl
└── work-config/
    ├── homie.toml
    └── .config/
        └── work-app/
            └── config.toml
```

The repo structure maps directly to the target:
- `~/.homie/repos/dotfiles/.zshrc` → `~/.zshrc`
- `~/.homie/repos/dotfiles/.config/nvim/init.lua` → `~/.config/nvim/init.lua`

Repos are discovered automatically - no global registration required.

### Link Strategies

| Strategy | Behavior |
|----------|----------|
| `file` | Symlink individual files (default) |
| `directory` | Symlink entire directory as a unit |
| `contents` | Create parent directory, symlink contents individually |
| `copy` | Copy file/directory instead of symlinking |

#### Copy Strategy

The `copy` strategy places files by copying rather than symlinking. This is useful for applications that don't follow symlinks or resolve them incorrectly.

**Behavior:**
- One-way sync: repo → target. Changes at target are overwritten on next `link`
- Always overwrites existing files (no conflict prompt)
- Preserves source file permissions (mode bits)
- Works with templates: `.tmpl` files are rendered then copied
- Works with imports: imported files can use copy strategy
- Dry run (`-n`) shows copy operations without executing

**File and directory level support:**
- Can copy entire directory trees: `".config/app" = "copy"`
- Can copy individual files within symlinked directories: `".config/app/settings.json" = "copy"`
- Supports glob patterns: `"*.json" = "copy"` matches at any depth

**Strategy changes:**
- Changing a path from symlink to copy (or vice versa) replaces transparently
- No need to unlink first; homie handles the transition

### External Symlinks

Symlinks in the target that point outside managed repositories. Homie can be configured to:
- **Preserve** symlinks not in `replaceable_paths` (never touch)
- **Replace** symlinks pointing to paths listed in `replaceable_paths`

**Example:** You have a dotfiles repo and a work project with its own config:

```
# Your dotfiles repo has:
~/.homie/repos/dotfiles/.config/app/config.toml

# But ~/.config/app/config.toml already points elsewhere:
~/.config/app/config.toml -> ~/dev/work-project/.config/app/config.toml
```

Without configuration, homie preserves the external symlink:
```
$ homie link
dotfiles:
  ⊘ .config/app/config.toml (external: ~/dev/work-project/.config/app/config.toml)
```

To allow homie to replace it, add the path to `replaceable_paths`:
```toml
# ~/.config/homie/config.toml
[settings]
replaceable_paths = ["~/dev/work-project"]
```

Now homie will replace the symlink with one pointing to your dotfiles repo.

### Imports

Repos can import files from external sources (local paths or git repos). Imported files are merged at root level, with the repo's own files taking precedence over imports.

```toml
# Import from local path
[[imports]]
source = "~/shared-dotfiles"

# Import from git repo
[[imports]]
source = "git@github.com:user/common-configs.git"
ref = "main"  # optional: branch, tag, or commit

# Import with explicit name
[[imports]]
name = "company-defaults"
source = "https://github.com/company/dotfiles.git"
```

**Behavior:**
- Local imports are used directly from the specified path
- Git imports are cloned to `<repo>/.homie/imports/<name>/`
- Git sources are auto-fetched on `homie link` (use `--no-fetch` to skip)
- Repo's own files always override imported files with the same path
- Imports are single-level (imports don't follow their own imports)

## Configuration

### Global Config (optional)

Location: `~/.config/homie/config.toml`

Only for global settings - no repo definitions:

```toml
[settings]
backup_suffix = ".backup.%Y%m%d%H%M%S"  # strftime format for backups

# Paths outside managed repos that are safe to replace
replaceable_paths = [
    "~/dev/other-project",
]

# Global variables available to all repos
[vars]
email = "user@example.com"
editor = "nvim"

# Environment variables to pass through to templates
[env]
pass_through = ["GITHUB_TOKEN", "OPENAI_API_KEY"]
```

### Per-Repo Config

Location: `<repo>/homie.toml`

```toml
# Required: where to link files (usually ~ for home directory)
target = "~"

# Optional: repo-specific variables (override globals)
[vars]
email = "user@work.com"

[defaults]
strategy = "file"  # file | directory | contents | copy

# Override strategy per path (relative to repo root)
# Supports explicit paths and glob patterns
[strategies]
".config/nvim" = "directory"
".local/bin" = "contents"
".config/app" = "copy"                    # copy entire directory
".config/other/settings.json" = "copy"   # copy single file
"*.plist" = "copy"                        # glob: matches at any depth

# Paths to ignore (in addition to defaults)
[ignore]
paths = ["*.swp", "temp/"]

# Optional: import files from external sources
[[imports]]
source = "~/shared-configs"

[[imports]]
source = "git@github.com:team/common-dotfiles.git"
ref = "main"
```

**Default ignored paths** (always ignored):
- `homie.toml`
- `.git/`
- `.homie/` (import cache and manifest)
- `README.md`, `README`
- `LICENSE`, `LICENSE.md`
- `.gitignore`

### Manifest

Homie tracks all managed files (symlinks and copies) in a manifest at `<repo>/.homie/manifest.toml`. This enables:
- Deleting copied files during `unlink`
- Detecting orphaned files (in manifest but not on disk)
- Tracking strategy changes

**Format:**
```toml
# Auto-generated by homie. Do not edit manually.
[files]
".zshrc" = "symlink"
".config/nvim" = "symlink"  # directory strategy
".config/app/settings.json" = "copy"
".gitconfig" = "rendered"   # template output
```

**Behavior:**
- Written at the end of `link` command (atomic update)
- Missing target files are silently removed from manifest
- Discrepancies shown as warnings in `homie status`

## Templates

Files with `.tmpl` extension are rendered before placement. The extension is stripped from the target filename.

### Syntax

```
{{variable}}              # Required variable
{{variable?}}             # Optional (empty string if missing)
{{variable:default}}      # Default value if missing
{{env.VARIABLE_NAME}}     # Environment variable (must be in pass_through list)
```

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `{{hostname}}` | Machine hostname |
| `{{user}}` | Current username |
| `{{home}}` | Home directory path |
| `{{os}}` | Operating system: `macos`, `linux`, or `windows` |

### Variable Resolution Order

1. Repo-specific vars (`[vars]` in repo's `homie.toml`)
2. Global vars (`[vars]` in global config)
3. Environment vars (`{{env.VAR}}` - must be in `pass_through`)
4. Built-in vars

### Example Template

`.gitconfig.tmpl`:
```ini
[user]
    name = {{user}}
    email = {{email}}

[github]
    token = {{env.GITHUB_TOKEN?}}

[core]
    editor = {{editor:vim}}
```

## CLI Interface

```
homie [OPTIONS] <COMMAND>

OPTIONS:
    -n, --dry-run    Show what would happen without making changes
    -v, --verbose    Verbose output
    -h, --help       Print help
    -V, --version    Print version

COMMANDS:
    link      Create symlinks for one or all repos
    unlink    Remove symlinks for one or all repos
    status    Show symlink status
    add       Add a file to a repo (move + symlink)
    diff      Show differences between repo and target
    init      Initialize a new repo
    clone     Clone an existing dotfiles repo
    list      List discovered repos
```

### Commands

#### `homie link [REPO] [--force] [--no-fetch]`

Create symlinks for one or all repos.

```
homie link                    # Link all repos
homie link dotfiles           # Link specific repo
homie link -n                 # Dry run
homie link --force            # Replace conflicts (with backup)
homie link --no-fetch         # Skip fetching git imports
```

Output:
```
dotfiles:
  ✓ .zshrc
  ✓ .config/nvim/
  ✓ .config/app/settings.json (copied)
  ⊘ .config/app/config (external: ~/dev/other)
  ⚠ .gitconfig (backup: .gitconfig.backup.20260110143022)
```

#### `homie unlink [REPO]`

Remove files managed by homie (symlinks and copies).

Uses the manifest to determine what to remove. Copied files are deleted, symlinks are removed.

#### `homie status [REPO]`

Show status for repos.

```
dotfiles (12 items):
  linked:    8
  copied:    2
  rendered:  1
  external:  2  (preserved, pointing outside repos)
  missing:   1  (in repo but not linked)
  conflict:  0
  ⚠ .config/old/file.txt in manifest but not on disk
```

#### `homie add <FILE> <REPO>`

Move a file into a repo and create a symlink.

```
homie add ~/.zshrc dotfiles
# Moves ~/.zshrc → ~/.homie/repos/dotfiles/.zshrc
# Creates symlink ~/.zshrc → ~/.homie/repos/dotfiles/.zshrc
```

#### `homie diff [REPO]`

Show files that differ between repo and target (for non-symlinked files).

#### `homie init <NAME> [--target <PATH>]`

Create a new repo at `~/.homie/repos/<name>/`.

```
$ homie init my-dotfiles
Creating repo my-dotfiles at ~/.homie/repos/my-dotfiles:
  ✓ Creating my-dotfiles/
  ✓ Creating homie.toml

$ homie init work-config --target ~/work
Creating repo work-config at ~/.homie/repos/work-config:
  ✓ Creating work-config/
  ✓ Creating homie.toml
```

#### `homie clone <URL> [--name <NAME>]`

Clone a git repo directly into `~/.homie/repos/`.

```
$ homie clone git@github.com:user/dotfiles.git
Cloning git@github.com:user/dotfiles.git into ~/.homie/repos/dotfiles:
  ✓ Cloned successfully

$ homie clone https://github.com/user/config.git --name myconfig
Cloning into ~/.homie/repos/myconfig:
  ✓ Cloned successfully
```

#### `homie list`

List all discovered repos.

```
Repos in ~/.homie/repos/:

  dotfiles (12 items)
    target: ~
    vars: email, editor

  work-config (3 items)
    target: ~
```

## Behavior Specifications

### Conflict Resolution

When target path exists and is not the expected symlink:

| Target State | Strategy | Action |
|--------------|----------|--------|
| Symlink → expected source | any | No-op (already correct) |
| Symlink → same repo | any | Replace with new link/copy |
| Symlink → replaceable external | any | Replace with repo link/copy |
| Symlink → other external | symlink | Skip, warn |
| Symlink → other external | copy | Replace (copy overwrites) |
| Symlink → broken | any | Replace with repo link/copy |
| Regular file/dir | symlink | Skip (use `--force` to backup and replace) |
| Regular file/dir | copy | Overwrite (always) |

### Backup Format

When `--force` is used, existing files are renamed with the configured suffix:
```
.gitconfig → .gitconfig.backup.20260110143022
```

Default format: `.backup.%Y%m%d%H%M%S`

### Template Rendering

- Templates (`.tmpl` files) are **rendered** to regular files, not symlinked
- Re-rendered on each `link` run
- If content matches existing file, no write occurs (idempotent)
- Missing required variables cause an error
- Missing optional variables (`{{var?}}`) render as empty string

### Idempotency

Running `homie link` multiple times produces the same result:
- Correct symlinks are skipped
- Templates with unchanged content are not rewritten
- Already-backed-up files are not re-backed-up
- Copied files with matching content are not recopied

### Glob Pattern Matching

Strategy glob patterns match files at any depth in the repo:
- `*.json` matches `.config/app/settings.json` and `data/config.json`
- `*.plist` matches any plist file anywhere in the repo

Patterns are matched against the full relative path from repo root. More specific patterns (explicit paths) take precedence over globs.

## File Structure

### Project Layout

```
homie/
├── Cargo.toml
└── src/
    ├── main.rs           # CLI entry point
    ├── config.rs         # Config parsing
    ├── repo.rs           # Repo discovery and iteration
    ├── import.rs         # External import handling
    ├── strategy.rs       # Link strategy enum (file, directory, contents, copy)
    ├── vars.rs           # Variable resolution
    ├── template.rs       # Template engine
    ├── linker.rs         # Core symlink and copy operations
    ├── manifest.rs       # Manifest reading/writing
    ├── status.rs         # Status checking
    └── commands/
        ├── mod.rs
        ├── link.rs
        ├── unlink.rs
        ├── status.rs
        ├── add.rs
        ├── diff.rs
        ├── init.rs
        ├── clone.rs
        └── list.rs
```

### Dependencies

- `clap` - CLI argument parsing
- `toml` / `serde` - Config file parsing
- `handlebars` - Template rendering
- `walkdir` - Directory traversal
- `colored` - Terminal colors
- `chrono` - Timestamp formatting
- `glob` - Pattern matching
- `shellexpand` - Tilde expansion
- `anyhow` - Error handling

## Migration from Other Tools

### From install.sh / custom scripts

1. Create repos at `~/.homie/repos/<name>/`
2. Add `homie.toml` to each repo with `target = "~"` and strategy overrides
3. Convert files needing variables to `.tmpl` format
4. Optionally create global config at `~/.config/homie/config.toml` for shared vars
5. Run `homie status` to verify detection
6. Run `homie link -n` to preview
7. Run `homie link` to apply
8. Remove old install script

### From homeshick

1. Move castles from `~/.homesick/repos/` to `~/.homie/repos/`
2. Flatten structure: move contents of `home/` up to repo root
3. Add `homie.toml` with `target = "~"` to each repo
4. Run `homie link`

### From GNU Stow

1. Move stow packages to `~/.homie/repos/`
2. Add `homie.toml` with appropriate target to each
3. Use `contents` strategy for stow-like behavior if needed
