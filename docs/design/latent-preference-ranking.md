---
title: "Design: Latent-Preference Ranking"
description: A redesign of jjj's personal and global ranking around directly-authored latent scores — sorted lists with sized gaps and on-demand named axes — with the math reserved for combination
status: Draft / brainstorm — not yet scheduled
date: 2026-05-30
---

# Design: Latent-Preference Ranking

> **Status:** Draft for discussion. No code committed. This captures a design
> direction explored in conversation; it is deliberately opinionated so it can
> be argued with. Nothing here is scheduled, and §9 lists what we would *not*
> do.
>
> **Revision note:** an earlier draft made *graded pairwise comparison* the core
> refinement mechanism. That has been demoted to an optional tie-break (§4.5).
> The core is now **a sorted list with sized gaps, plus on-demand named axes** —
> simpler, keystroke-cheap, and a better fit for jjj's history (§2).

## 1. Motivation

The current ranking system (tier sort + quadratic votes, aggregated by
normalized harmonic ordering + QV — see `docs/guides/ranking.md` and
`src/ranking/scoring.rs`) works, but pushing on it surfaced a real tension:

- **What I'm expressing is one quantity, not two.** Between items I have a
  *signed magnitude* — direction (which I prefer) and intensity (by how much)
  are the same feeling. The current model splits these into two mechanisms:
  **ordering** (purely ordinal: tier sort, bubble) and **votes** (the intensity
  channel: QV). That split is the friction.
- **I can't introspect an absolute value.** Asking me to *state* a rank or a
  vote budget over the whole list demands I hold everything in my head at once.
  I can only feel *local* relationships — "this is a cut above that."
- **It's probably not 1-D.** My preferences are shaped by axes (impact, effort,
  risk, personal interest…) I can't enumerate up front and that may not collapse
  cleanly into one order.
- **Scale is asymmetric.** I want to triage *hundreds* of items fast, then
  resolve nuance only at the **extremes** — the few at the top that truly matter,
  and the few at the bottom that *must not win*. The middle can stay mushy.

This doc proposes treating ranking as **direct, local, incremental authoring of
a latent score** — never a global "state your number" act — with all the
genuinely fancy math deferred to *combination* (across a user's axes, and across
users).

## 2. Prior art in this repo (why the core is *not* a comparison engine)

**jjj already had a full pairwise/rating ranker and removed it.** This is
documented in git, and it is the reason the core mechanism below is a list, not
a comparison flow.

History (verified):

- A Glicko-2 pairwise system was built across `bf95831 → 3407d57`:
  `src/ranking/glicko2.rs` (472 lines), `matchups.rs` (uncertainty-driven
  matchup suggestion), `store.rs` (per-user/per-milestone JSONL), a 599-line
  `rank` command, and a TUI overlay. Design: `docs/plans/2026-03-17-pairwise-ranking-design.md`.
- It was removed in `2443fc1` ("remove Glicko-2 ranking system") and replaced by
  the manual-ordering + QV branch. Design: `docs/plans/2026-03-22-manual-ordering-qv-design.md`.
- The `*.jsonl`-skipping branch in `load_all_orderings` is the last trace of it.

**The documented reason for removal** (verbatim, from the replacement design's
Problem section):

> "Pairwise ranking (Glicko-2) requires many comparison sessions to converge, is
> slow for large lists, and feels disconnected from the tree view."

So the three real failure modes were **(a) slow convergence**, **(b) slow for
large lists**, and **(c) UX disconnection** (the comparison flow lived outside
the tree view). The list-with-gaps core (§3) beats all three *more cleanly than
any comparison-based design could*:

- *(a) convergence* — **zero comparison sessions.** Gaps and axes are direct
  authoring; the score converges the instant you stop typing. There is nothing
  to iterate to.
- *(b) scale* — gaps and splits are **sparse, optional annotations** on the list
  you already have. At triage scale you just sort tiers; you mark a cliff only
  where you care.
- *(c) UX disconnection* — it **is** the list/tree view. No separate mode.

Pairwise is not gone, but it is demoted to an optional tie-break for the handful
of items you genuinely can't separate (§4.5) — exactly the narrow case where it
helps, never the primary loop. If we ever find ourselves rebuilding a comparison
*session*, we are re-deriving the 2026-03 removal.

## 3. Core idea

Each item `i` has a latent score `sᵢ`. **Direction = sign(sᵢ − sⱼ); intensity =
|sᵢ − sⱼ|** — two readouts of one number, collapsing the ordering/votes split.

The user authors `s` directly and locally, with **one primitive used at two
levels**: a *sorted list with sized gaps*. Fidelity scales with how much they
care:

| Zoom | Items | Action | Output |
|------|-------|--------|--------|
| **Triage** | hundreds | Tier sort (today's UI, unchanged) | coarse order |
| **Intensity** | wherever there's a cliff | Insert a sized **gap** (S/M/L/XL) between neighbors | a per-axis score `sᵢ` |
| **Dimensionality** | when you feel torn | **Split**: name an axis, re-sort the same items | a vector `(sᵢ⁽ᵃ⁾)` per item |
| **Combine** | automatic | weighted, normalized sum (§4.3) | one ranking |

Nothing here is a comparison session. You sort, you mark the cliffs, and when one
list can't hold your feelings you split out the axis you're torn about. The math
only shows up when combining axes and users.

## 4. The math (mostly in combination, not authoring)

### 4.1 Gaps → a directly-authored 1-D score

A sorted list with sized gaps **is** a 1-D embedding, authored directly. For the
list top-to-bottom, let `gₖ ≥ 0` be the gap *below* the item at rank `k`, chosen
from a small discrete set:

```
Unit (default, no annotation),  S,  M,  L,  XL   →   numeric, e.g. 1, 2, 4, 8, 16
```

The score is the cumulative descent:

```
s_{rank=1} = 0 ;   s_{rank=k} = − Σ_{j<k} gⱼ
```

Only differences matter (the absolute zero is free gauge). Two properties:

- **Intensity for free, no second mechanism.** The gap *is* the magnitude the
  ordinal list couldn't express. "This tier is a different league" = one XL gap.
- **Exact backward-compat.** Every gap = `Unit` ⟹ sorted-by-score is identical
  to today's list order. This is a stronger compatibility guarantee than the
  previous draft's approximate `λ→∞` argument — it's an *equality*, not a limit.

Authoring a gap is a **local** judgment — you eyeball one boundary between two
neighbors and say "big cliff / small cliff." You never hold the whole list in
your head, which is the whole point of §1.

**"Must-not-win" falls out naturally.** A large gap *above* the bottom item says
"this is in a worse league," expressing the asymmetric aversion that previously
needed negative QV votes — and it's just another gap, no special channel.

### 4.2 Split → named axes (dimensionality by declaration, not inference)

When one list can't hold your feeling — you keep flip-flopping two items because
you're trading off two things — you **split**: name the axis ("price"), and
re-sort the *same* items as an independent list-with-gaps. Repeat for "quality",
etc. Each axis `a` yields its own directly-authored score `sᵢ⁽ᵃ⁾`.

This is the crucial simplification. The previous draft tried to *recover* hidden
axes from your intransitivity via a low-rank skew model — which, as §4.4 notes,
is only weakly identifiable from one person. **Declaring** the axis at the moment
of strain sidesteps that entirely:

- The intransitivity that couldn't fit on one line ("cheap but bad" vs "great but
  pricey") now lives naturally as *high on one axis, low on another*.
- You only ever get axes you can name — which is honest, because unnameable
  latent structure isn't reliably recoverable from one user anyway (§4.4).
- On the first split, the original list becomes the first named axis (prompt:
  "what was that list about — gut feel? overall?").

### 4.3 Combination — the one place the math lives

Within a user with axes `a`, combine the per-axis scores into one ranking:

```
sᵢ = Σₐ  βₐ · z(sᵢ⁽ᵃ⁾)
```

Two deliberately-separated signals:

- **`z(·)` normalizes *within* each axis** (e.g. z-score or scale-to-unit-range).
  This strips out "I happened to use bigger cliffs on price." Per-axis *spread*
  must not masquerade as importance.
- **`βₐ` carries cross-axis importance** — and here's the self-similar trick:
  **you set `βₐ` by ranking the axes themselves, with gaps.** The axes are just
  items in a meta-list-with-gaps, so §4.1 gives the weights. One primitive, two
  levels; nothing new to learn.

This is trivial, robust, and explainable, and it degrades gracefully: one axis →
just that list; no splits and all-`Unit` gaps → exactly today's behavior.

### 4.4 Why we declare axes instead of inferring them (identifiability)

A *single* user's transitive preferences are intrinsically ~1-D-identifiable —
one number per item explains any transitive set. Extra preference dimensions are
recoverable only through *intransitivity* (a cyclic residual), through item
features, or through multiple users. So blind multi-axis recovery from one
person's judgments is a weak bet. The Hodge "rankability" decomposition
(`M = grad(s) + cyclic`) is still useful — but only as an **advisory nudge**
(§4.5), not as an extraction engine. Declaration (§4.2) is the reliable route.

### 4.5 Demoted to optional: pairwise tie-break + split nudge

Two narrow, *optional* roles remain for comparison machinery — never a session,
never the primary loop:

- **Tie-break.** When two items sit at the same score (no gap) and the user keeps
  swapping them, offer: "settle these with 3 quick comparisons?" This is the only
  place the Laplacian fit / effective-resistance sampling from the previous draft
  earns its keep — as a micro-tool for the hardest 2–3 local calls, typically at
  the very top.
- **Split nudge.** If the user repeatedly reorders the same small cluster (a
  behavioral proxy for the cyclic residual of §4.4), gently suggest: "you keep
  flip-flopping these — want to split out an axis?" The system *detects* the
  strain; the user *declares* the axis. Advisory, dismissible.

If neither is built, the core (§4.1–4.3) stands on its own.

## 5. Personal vs. global — the key separation

> **Budgets are a strategy-resistance tool for aggregation, not an elicitation
> tool for the individual.**

- **Personal view = direct authoring.** No budget, no ordering-vs-votes split.
  Just per-axis lists-with-gaps and the combined score. You have no incentive to
  game yourself, so a budget here is pure friction.
- **Global view = fair combination under strategic pressure.** This is where QV
  budgets and harmonic normalization earn their keep (anti-grief, equal baseline
  influence across users) — what `aggregate_rankings` does today.

Each user now contributes, per axis, a normalized score vector. Aggregation:

- **Normalize and pool per-axis scores** across users (budget-gated, as today).
- **Align axes across users.** Do two users' "price" lists agree? And do users
  even *share* axes — maybe Alice splits price/quality while Bob splits
  speed/risk? That disagreement *about what the dimensions are* is itself a
  first-class finding.
- **Surface per-item, per-axis cross-user variance** as the contention signal;
  point group discussion at the highest-variance items. Bimodal/"barbell"
  variance = polarization, more actionable than a scalar.

## 6. Data model & compatibility

Today: `rankings/{milestone_id}/{user_slug}.json` holding `UserOrdering { order:
Vec<String>, votes: HashMap<String,i32>, updated_at }`, aggregated by
`aggregate_rankings(orderings, problem_count)` →
`AggregatedRank { position, score, voter_count }`.

The migration is **strictly additive** — every new field `#[serde(default)]`, so
old files parse unchanged and an un-annotated file reproduces today's behavior
*exactly* (§4.1):

```rust
struct UserOrdering {
    order: Vec<String>,                 // existing — the primary/gut axis order
    votes: HashMap<String, i32>,        // existing — retained for back-compat
    #[serde(default)] gaps: Vec<GapSize>,        // NEW: gap below order[k]; default Unit
    #[serde(default)] axes: Vec<Axis>,           // NEW: extra named axes
    #[serde(default)] axis_order: Vec<String>,   // NEW: meta-ranking of axes (for β)
    #[serde(default)] axis_gaps: Vec<GapSize>,   // NEW: gaps on the meta-ranking
    updated_at: DateTime<Utc>,
}
struct Axis { name: String, order: Vec<String>, gaps: Vec<GapSize> }
enum GapSize { Unit, S, M, L, XL }      // Unit serializes as absent/default
```

Notes:

1. **`order` + `votes` stay the source of truth for single-axis use.** When
   `gaps` is empty (all `Unit`) and `axes` is empty, scoring == current behavior.
   `votes` can be reinterpreted later as a coarse gap shorthand, or kept as-is.
2. **Reuse the file format and sync path** — same path, same atomic write, same
   `jj git push`. No new storage surface.
3. **The `.jsonl` Glicko-2 files stay ignored.** We do not resurrect that format.

## 7. Phased plan

Each phase is independently shippable and independently abandonable.

1. **Gaps on the primary list.** Add `gaps` + the cumulative-descent score
   (§4.1); a TUI keybind to set/clear S/M/L/XL on the boundary at the cursor.
   Prove all-`Unit` reproduces current order. *Delivers intensity — the headline
   half of the ask — with one new field and one keybinding.*
2. **Split into named axes.** Add `axes`, the split/name/resort flow, the
   meta-ranking for `β`, and the normalized combination (§4.3). *Delivers
   dimensionality by declaration.*
3. **Global: pool per-axis scores; expose contention.** Augment
   `aggregate_rankings` to combine normalized per-axis vectors; keep QV budget +
   harmonic normalization for strategy-resistance; add the axis-alignment and
   per-axis variance views (§5).
4. **Optional tie-break.** The 3-comparison micro-tool for unseparated items
   (§4.5) — only if users actually hit ties they want resolved.
5. **Optional split nudge.** Detect flip-flop clusters and suggest a split (§4.5)
   — pure advisory polish.

Stop after phase 1 if intensity alone satisfies. Phase 2 covers
multidimensionality. Phases 4–5 are conveniences, not load-bearing.

## 8. Open questions

- **Gap calibration.** What numeric values for S/M/L/XL — geometric (1/2/4/8/16)
  or linear (1/2/3/4/5)? Four levels feels right (avoids false precision); is a
  fifth (XL) needed for the "different league" case?
- **Within-axis normalization.** z-score, scale-to-range, or rank-normalize? The
  choice changes how much an axis's *spread* survives into the combination — and
  we explicitly want spread separated from importance `β` (§4.3).
- **Axis proliferation.** Do users over-split? Probably self-limiting (effort),
  but watch for it; consider a merge gesture.
- **Do axes carry to the group, or stay personal?** If users don't share axis
  names, alignment becomes fuzzy matching ("price" vs "cost"). Treat divergent
  axes as a finding (§5) or try to reconcile them?
- **Tie-break scope.** Is the pairwise micro-tool worth building at all, or does
  "no gap = genuinely tied, leave it" suffice?

## 9. Non-goals / explicitly rejected

- **No comparison *session* as the core.** Pairwise survives only as an optional
  tie-break/nudge (§4.5). Rebuilding a session re-creates the Glicko-2 removal
  (§2).
- **No 2-D "drag the orbs" canvas.** It breaks at hundreds of items and demands
  global spatial authoring. Acceptable only as a Resolve-layer deliberation
  surface for the final handful, never as triage.
- **No multiplicative harmonic × quadratic weighting** (`k_{u,i}/r_{u,i}`). It
  suppresses the high-intensity-low-rank "must-not-win" signal. Additive,
  same-scale combination stays.
- **No streaming rating model (Glicko-2/Elo)** for the fixed item set.
- **No budget on personal ranking.** Budgets live at aggregation only (§5).
- **No blind multi-axis inference.** Axes are *declared* (§4.2), not extracted —
  one user's preferences don't reliably identify hidden axes (§4.4).

## 10. Stolen-with-credit ideas

From a Gemini brainstorm, the parts worth keeping — all as *group/Resolve-layer*
tools on top of the per-axis authoring, never as replacements for it:

- **Generalized Procrustes Analysis** to align subjective per-user frames — the
  multi-user generalization of within-axis normalization / gauge-fixing.
- **Variance "nebula" / bimodality** as a polarization read.
- **PCA / emergent axis naming** — a softer assist to the *declare-an-axis* flow
  (§4.2): suggest a tentative name when a split's items cluster.
- **Socratic targeting of highest-variance items** for discussion — the single
  best transplant; free once we compute per-axis cross-user variance (§5).
- **Embodied cost** (a draining budget bar) as a UX cue at aggregation time.
