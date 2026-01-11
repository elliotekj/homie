# Tweet Thread: Homie Initial Release

---

**Tweet 1 (Hook)**

Just shipped homie — a dotfiles symlink orchestrator built in Rust.

It solves the problem I've had for years: managing dotfiles across multiple repos without everything becoming a tangled mess.

Here's why I built it and how it works 🧵

---

**Tweet 2 (The Problem)**

Most dotfile tools assume one repo, one machine, simple needs.

Real life: I have personal configs, work configs, and team-shared configs. Some files need different values per machine. Some directories should stay together, others should merge.

GNU Stow wasn't cutting it.

---

**Tweet 3 (Multi-repo)**

Homie is multi-repo by design.

Drop repos in ~/.homie/repos/, each with a homie.toml config. Run `homie link`. Done.

Each repo is independent. Your personal dotfiles stay separate from work configs. Clean precedence when they overlap.

---

**Tweet 4 (Linking Strategies)**

Three linking strategies:

• file (default) — each file gets its own symlink
• directory — entire dir becomes one symlink (great for nvim config)
• contents — parent dir created, contents merged (perfect for ~/.local/bin)

Configure per path. No more hacks.

---

**Tweet 5 (Templates)**

Files ending in .tmpl get rendered with Handlebars before linking.

.gitconfig.tmpl → ~/.gitconfig

Built-in vars: hostname, user, home, os
Custom vars in config
Optional syntax: {{var?}}, {{var:default}}

No preprocessing scripts needed.

---

**Tweet 6 (Imports)**

Import configs from git repos or local paths:

```toml
[[imports]]
source = "https://github.com/team/shared-configs.git"
paths = [".zshrc", ".config/git/*"]
remap = [{ from = "commands", to = ".claude/commands" }]
```

Your own files take precedence. Auto-fetches on link.

---

**Tweet 7 (Smart Conflicts)**

Homie preserves external symlinks by default.

If something's already symlinked to a location outside your managed repos, it won't blow it away.

--force creates timestamped backups. -n/--dry-run shows what would happen first.

---

**Tweet 8 (CLI)**

```
homie init dotfiles    # create new repo
homie add dotfiles ~/.zshrc  # move file in
homie link             # symlink everything
homie status           # see what's linked
homie diff             # show differences
```

Single binary. No shell scripts. Pure Rust.

---

**Tweet 9 (CTA)**

If you're tired of:
- Scattered install scripts
- One-size-fits-all dotfile tools
- Manual symlink management

Give homie a try.

GitHub: [LINK]

PRs welcome. Let me know what you'd add.

---
