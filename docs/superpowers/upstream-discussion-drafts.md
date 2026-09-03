# Upstream Discussion drafts

Herdr does not accept unsolicited pull requests (see `CONTRIBUTING.md`). The
sanctioned route for feature work is a GitHub Discussion, written for humans,
that explains the problem and links the working branch. Two drafts follow, one
per feature. Existing threads to post in or reference:

- Focus follows mouse: https://github.com/herdrdev/herdr/discussions/1259 and
  https://github.com/herdrdev/herdr/discussions/2230
- Per-space colour: https://github.com/herdrdev/herdr/discussions/1942

Branch: `https://github.com/potable-anarchy/herdr/tree/feature/focus-follows-mouse-and-space-themes`

---

## Draft 1: reply on #1259 (focus follows mouse)

I run Herdr under a focus-follows-mouse window manager, so every time I move
between panes I still have to click, and the click sometimes lands in the
running agent's TUI. I have this working as an opt-in setting in my fork and
have been using it daily.

What it does:

- `ui.focus_follows_mouse = false` by default; also a toggle under Settings → focus.
- Moving the pointer into a pane focuses it. Sidebar, tab bar, and agents panel
  still need a click.
- It sends the same `pane.focus` request a click does, only when the hovered
  pane is not already focused, and only in terminal mode with no overlay, drag,
  or pane gesture in progress. Button-held motion arrives as drag, so text
  selection and split resizing keep the pane they started in.
- No hover highlighting.

Branch, with tests for every guard:
https://github.com/potable-anarchy/herdr/tree/feature/focus-follows-mouse-and-space-themes

Happy to leave it there as a reference, or to trim it to whatever shape you'd
want if you pick this up.

---

## Draft 2: new Discussion (per-space theme)

**Title:** Per-space theme override, chosen from the space's right-click menu

**Problem.** With four or five spaces open for different projects I lose track
of which one I'm looking at. The global theme can't distinguish them, and the
sidebar row tokens are text only. #1942 asked for a per-space accent colour for
the same reason.

**What I'd want.** Right-click a space → Theme... → pick one of the built-in
themes, with a "use global theme" entry to clear it. The chosen theme colours
that space's pane borders and titles, the tab bar while it is active, and its
own sidebar row (text colours plus a one-column accent bar at the leading edge,
visible whether or not the space is active). Everything else stays on the
global theme. New spaces follow the global theme until overridden, so
changing the global theme still changes every space that has no override.

**Scope I deliberately kept out.** Pane content colours (those come from the
host terminal), hover effects, and any config-file persistence; the override
is stored with the space in the session file and set through a small
`workspace.set_theme` endpoint method.

I have this built and in daily use in my fork, with server and client tests:
https://github.com/potable-anarchy/herdr/tree/feature/focus-follows-mouse-and-space-themes

Posting here rather than as a PR per the contribution policy. If the idea fits
Herdr, the branch is there to take or to point at.
