# Homie

Dotfiles symlink orchestrator with templates and multiple repo support.

> [!WARNING]
> I am using this project for [my own dotfiles](https://github.com/elliotekj/dotfiles), but the project is in its early days.

## Features

- **Multi-repo management** - Organize dotfiles across multiple repositories
- **Three linking strategies** - Link files, directories, or directory contents
- **Template support** - Render `.tmpl` files with variable substitution
- **Git/local imports** - Pull files from external git repos or local paths
- **Smart conflict resolution** - Preserve external symlinks, backup on force
- **Dry-run mode** - Preview changes before applying

## Quick Start

```bash
# Build and install
cargo install --path .

# Initialize your first repo
homie init dotfiles

# Add a dotfile (moves file and creates symlink)
homie add dotfiles ~/.zshrc

# Link everything
homie link
```

## How It Works

Homie stores dotfile repos in `~/.homie/repos/`. Each repo contains a `homie.toml` config and your dotfiles in the same directory structure as their target location.

```
~/.homie/repos/
└── dotfiles/
    ├── homie.toml          # repo config
    ├── .zshrc              # links to ~/.zshrc
    ├── .gitconfig.tmpl     # renders then links to ~/.gitconfig
    └── .config/
        └── nvim/
            └── init.lua    # links to ~/.config/nvim/init.lua
```

<details>
<summary><h2>Configuration Reference</h2></summary>

### Global Config

Optional config at `~/.config/homie/config.toml` for shared settings:

```toml
[settings]
# Suffix for backup files (strftime format)
backup_suffix = ".backup.%Y%m%d%H%M%S"

# Paths that can be replaced even if they're external symlinks
replaceable_paths = ["~/some/generated/path"]

[vars]
# Variables available in all repos
email = "you@example.com"
github_user = "yourusername"

[env]
# Environment variables to expose in templates as {{env.VAR_NAME}}
pass_through = ["WORK_EMAIL", "API_KEY"]
```

### Repo Config

Required config at `<repo>/homie.toml`:

```toml
# Required: where symlinks point to
target = "~"

[vars]
# Repo-specific variables (override global vars)
git_user = "work-account"

[defaults]
# Default linking strategy: "file", "directory", or "contents"
strategy = "file"

[strategies]
# Override strategy for specific paths
".config/nvim" = "directory"    # link entire directory
".local/bin" = "contents"       # link parent, contents individually

[ignore]
# Additional paths to ignore (glob patterns)
paths = ["*.swp", "scratch/**"]

# External file imports
[[imports]]
source = "https://github.com/user/shared-configs.git"
ref = "main"           # optional: branch, tag, or commit
name = "shared"        # optional: defaults to repo name
paths = ["*"]          # optional: paths to include (default: all)
remap = []             # optional: path remapping rules

[[imports]]
source = "~/work/dotfiles"    # local path import
paths = [".zshrc", ".gitconfig"]  # selective import
```

### Default Ignores

These paths are always ignored:
- `homie.toml`, `.git`, `.homie`, `.DS_Store`
- `README.md`, `README`, `LICENSE`, `LICENSE.md`, `.gitignore`

</details>

<details>
<summary><h2>Linking Strategies</h2></summary>

### File (default)

Each file gets its own symlink. Best for most dotfiles.

```
Repo:                        Target:
.zshrc          →           ~/.zshrc (symlink)
.config/git/config  →       ~/.config/git/config (symlink)
```

### Directory

The entire directory becomes a single symlink. Use for directories that should stay together (e.g., nvim config, application bundles).

```toml
[strategies]
".config/nvim" = "directory"
```

```
Repo:                        Target:
.config/nvim/       →       ~/.config/nvim (symlink to entire dir)
  ├── init.lua
  └── lua/
```

### Contents

The parent directory is created, then each item inside is linked individually. Use when you want to merge files into an existing directory.

```toml
[strategies]
".local/bin" = "contents"
```

```
Repo:                        Target:
.local/bin/         →       ~/.local/bin/ (real dir)
  ├── script1               ~/.local/bin/script1 (symlink)
  └── script2               ~/.local/bin/script2 (symlink)
```

</details>

<details>
<summary><h2>Templates</h2></summary>

Files ending in `.tmpl` are rendered with Handlebars before linking. The `.tmpl` extension is removed in the target.

### Syntax

```
{{var}}           Required variable - error if missing
{{var?}}          Optional - empty string if missing
{{var:default}}   Default value if missing
{{env.VAR_NAME}}  Environment variable (must be in pass_through list)
```

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `{{hostname}}` | Machine hostname |
| `{{user}}` | Current username |
| `{{home}}` | Home directory path |
| `{{os}}` | Operating system: `macos`, `linux`, or `windows` |

### Example Template

`.gitconfig.tmpl`:
```ini
[user]
    name = {{git_name}}
    email = {{email}}

[github]
    user = {{github_user?}}

[core]
    editor = {{editor:vim}}
```

### Variable Resolution Order

1. Repo-specific vars (from `homie.toml`)
2. Global vars (from `~/.config/homie/config.toml`)
3. Environment vars (via `env.pass_through`)
4. Built-in vars

</details>

<details>
<summary><h2>Imports</h2></summary>

Import files from external sources to include in your repo's linking.

### Git Import

```toml
[[imports]]
source = "https://github.com/user/shared-dotfiles.git"
ref = "main"        # branch, tag, or commit (optional)
name = "shared"     # directory name (optional, derived from URL)
paths = ["*"]       # paths to include (optional, default: all)
```

Git imports are cloned to `<repo>/.homie/imports/<name>/` and pulled on each `homie link`.

### Local Import

```toml
[[imports]]
source = "~/work/company-dotfiles"
name = "work"       # optional
```

### Selective Paths

By default, all files from an import are included (`paths = ["*"]`). Use `paths` to import only specific files or directories:

```toml
[[imports]]
source = "https://github.com/user/dotfiles.git"
paths = [".zshrc", ".config/nvim"]  # only these paths

[[imports]]
source = "~/shared"
paths = [".config/*"]               # glob patterns supported
```

When a directory is specified (e.g., `.config/nvim`), all files within it are included.

### Path Remapping

Remap import paths to different target locations using `remap`:

```toml
[[imports]]
source = "https://github.com/user/claude-commands.git"
paths = ["commands/**"]
remap = [{ from = "commands", to = ".claude/commands" }]
```

This imports files from `commands/` in the source repo but links them to `.claude/commands/` in the target. Useful when the source repo structure doesn't match your desired target layout.

### Import Precedence

Repo's own files take precedence over imported files. If both your repo and an import have `.zshrc`, your repo's version wins.

### Skip Fetching

Use `--no-fetch` to skip pulling git imports:

```bash
homie link --no-fetch
```

</details>

<details>
<summary><h2>CLI Reference</h2></summary>

### Global Options

```
-n, --dry-run    Show what would happen without making changes
-v, --verbose    Verbose output
-h, --help       Print help
-V, --version    Print version
```

### Commands

#### `homie link [REPO]`

Create symlinks for one or all repos.

```bash
homie link              # link all repos
homie link dotfiles     # link specific repo
homie link --force      # backup conflicts and replace
homie link --no-fetch   # skip pulling git imports
homie link -n           # dry run
```

#### `homie unlink [REPO]`

Remove symlinks managed by homie.

```bash
homie unlink            # unlink all repos
homie unlink dotfiles   # unlink specific repo
```

#### `homie status [REPO]`

Show the state of each symlink.

```bash
homie status
homie status -v         # verbose (show all files)
```

Status indicators:
- `✓` linked correctly
- `~` rendered template
- `→` external symlink (not managed)
- `✗` missing or conflict

#### `homie add <REPO> <FILE>`

Move a file into a repo and create a symlink in its place.

```bash
homie add dotfiles ~/.zshrc
homie add dotfiles ~/.config/nvim
```

#### `homie diff [REPO]`

Show differences between repo files and their targets.

```bash
homie diff
homie diff dotfiles
```

#### `homie init <NAME>`

Create a new repo with boilerplate config.

```bash
homie init dotfiles
homie init work --target ~/work    # custom target directory
```

#### `homie clone <URL>`

Clone an existing dotfiles repo.

```bash
homie clone https://github.com/user/dotfiles.git
homie clone git@github.com:user/dotfiles.git --name my-dotfiles
```

#### `homie list`

List all discovered repos.

```bash
homie list
```

</details>

## Example Workflow

```bash
# Start fresh
homie init dotfiles

# Add your existing dotfiles
homie add dotfiles ~/.zshrc
homie add dotfiles ~/.gitconfig
homie add dotfiles ~/.config/nvim

# Check status
homie status

# On a new machine, clone and link
homie clone https://github.com/you/dotfiles.git
homie link

# Preview changes after editing
homie link -n

# Force link with backups if conflicts exist
homie link --force
```

## License

`homie` is released under the [Apache License 2.0](LICENSE).

## About

This package was written by [Elliot Jackson](https://elliotekj.com).

- Blog: [https://elliotekj.com](https://elliotekj.com)
- Email: elliot@elliotekj.com
