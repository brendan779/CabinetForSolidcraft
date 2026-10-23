# Overnight runbook

How an unattended Claude session works on this repository while the owner is away. The session
works through [BACKLOG.md](BACKLOG.md), one item at a time, and leaves every change on a branch
for the owner to review in the morning.

## The loop

1. **Start clean.** `git fetch origin && git switch main && git pull --ff-only`, then create
   `night/<YYYY-MM-DD>` (or switch to it if it exists). Run `scripts/check.sh`. If main is red,
   fixing it is the first task.
2. **Pick an item.** Take the first `todo` row in the highest priority section whose dependencies
   are done and that is not marked 👤 / `needs-human`. Set it to `doing (night/<date>)` in
   BACKLOG.md.
3. **Do it.** Small steps, tests first where the item has numbers in it. Match the surrounding
   code (see [CLAUDE.md](../CLAUDE.md)).
4. **Prove it.** `scripts/check.sh` must be green. If the item touches SolidCraft output, also run
   the live check below. For UI items, build the app, launch it, capture a screenshot into
   `docs/images/` and look at it.
5. **Commit.** One commit per item: `<ID>: <what changed>`. Set the row to `done (<short sha>)`
   and add a line to the Log section.
6. **Repeat** until the time box ends (default 6 hours) or three items in a row end `blocked`.
7. **Hand over.** Push the branch and open a draft PR titled `Overnight <date>`. Its body lists
   items done, items blocked and why, screenshots, and anything the owner must decide.

## Rules

- **Never push to `main`.** Never force-push. Never rewrite history.
- **Never commit supplier files.** Nothing from `vendor-cad/`, no catalogue PDFs, no swatch
  images. Facts (dimensions, article numbers, sheet sizes) go into `catalog/*.json` with their
  `source`. Never mark data `verified: true`: only the owner does that.
- **Don't touch the owner's SolidCraft work.** The live check may only build into a design that
  is untitled and empty (`document.inspect`: no bodies, no path). Otherwise skip the live check and
  say so in the PR. Never call `doc.*` or `file.*` commands, and never close or save the owner's
  designs.
- **No new external services, accounts or paid tools.** Homebrew/cargo packages that are widely
  used are fine. Record them in the PR.
- **When unsure, write it down instead of guessing.** Add a `needs-human` row with the question.
- Keep `scripts/check.sh` fast. A step that takes more than 2 minutes needs an opt-in flag.

## Live check against SolidCraft

Prefer the **headless** check. It uses the CLI inside the installed app and never touches the
owner's open designs:

```bash
cargo run -q -p solidcraft-bridge --example emit_base -- --script target/run.json
/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli run target/run.json --out target/run.step
```

Add `--manufacturing` to cut every bore and slot (slow until PERF-1 is done). The connected
check (`emit_base` without `--script`) drives the running app on port 7878. Use it only under the
rule above.

## Starting a night by hand

From the repository root in Claude Code:

```text
Work through docs/BACKLOG.md following docs/OVERNIGHT.md for the next 6 hours.
```

A scheduled task can send the same prompt each night. See the README.
