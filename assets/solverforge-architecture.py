#!/usr/bin/env python3
"""Generate the SolverForge 'system wiring at a glance' architecture diagram.

Emits the standalone HTML document with inline SVG — the single source both
committed PNGs are exported from — and, when a rendered PNG is already present,
keeps the README copy in sync with it.

Palette and typography follow the solverforge.org dark surfaces and the
solverforge-ui design tokens; the ouroboros emblem and wordmark are inlined from
the site brand assets.

Run from anywhere:

    python3 assets/solverforge-architecture.py

The PNG exports are rendered from the emitted HTML in a browser (the HTML uses
only system-renderable SVG, so a full-page screenshot of the `.diagram` element
is the export). Re-render into docs/architecture.png after changing geometry,
and this script copies it to assets/solverforge-architecture.png for the README.

Optional: pass a repository root or the HTML output path as the first argument.
"""
import html
import pathlib
import shutil
import sys

_REPO = pathlib.Path(__file__).resolve().parent.parent
if len(sys.argv) > 2:
    raise SystemExit("usage: solverforge-architecture.py [repo-root-or-output-path]")
if len(sys.argv) == 2:
    _arg = pathlib.Path(sys.argv[1]).resolve()
    _REPO = _arg.parent if _arg.suffix else _arg

OUT = _REPO / "docs" / "architecture.html"
PNG = _REPO / "docs" / "architecture.png"
README_PNG = _REPO / "assets" / "solverforge-architecture.png"

# ── Brand constants (solverforge.org index.scss + solverforge-ui 00-tokens.css) ──
BG = "#051e1a"                 # deep emerald-night, brand-adjacent
PANEL = "#083029"              # panel background (dark surface family)
GRID_STROKE = "#6ee7b7"        # emerald-300, used at very low opacity like the brand
EM300, EM400, EM500, EM600 = "#6ee7b7", "#34d399", "#10b981", "#059669"

INK, INK_SOFT = "#cfe9df", "#8fb5a8"      # primary / secondary body text
INK_T = "#ecfdf5"                          # strong text
INK_MUTED = "#5e8a7e"                      # faint text
BORDER = "rgba(110,231,183,0.17)"
BORDER_S = "rgba(110,231,183,0.24)"

# Semantic accents (stored per node)
CY, VI, EM, AM, RO, SL = "cyan", "violet", "emerald", "amber", "rose", "slate"
NODE = {
    CY: {"stroke": "#5eead4", "fill": "rgba(20,184,166,0.10)", "title": "#99f6e4", "dot": "#2dd4bf"},   # model / entry — teal
    VI: {"stroke": "#a78bfa", "fill": "rgba(167,139,250,0.10)", "title": "#c4b5fd", "dot": "#a78bfa"},   # core types + scoring
    EM: {"stroke": EM300,     "fill": "rgba(16,185,129,0.12)",  "title": "#a7f3d0", "dot": EM400},       # solver engine — the brand accent
    AM: {"stroke": "#fcd34d", "fill": "rgba(234,179,8,0.10)",   "title": "#fde68a", "dot": "#eab308"},   # config — brand gold
    RO: {"stroke": "#fb7185", "fill": "rgba(251,113,133,0.10)", "title": "#fda4af", "dot": "#fb7185"},   # runtime invariants
    SL: {"stroke": "#6ea99b", "fill": "rgba(110,169,155,0.08)", "title": "#c6e2d8", "dot": "#6ea99b"},   # neutral
}
# Boundary frames echo the section's dominant accent.
BAND = {CY: "#5eead4", VI: "#a78bfa", EM: EM300, RO: "#fb7185", AM: "#fcd34d", SL: "#6ea99b"}
# Flow/marker colors used by edges.
FLOW = {"slate": "#6ea99b", "emerald": EM400, "rose": "#fb7185", "violet": "#a78bfa", "amber": "#fcd34d"}
MARKER = {c: f"ar-{name}" for name, c in FLOW.items()}

FONT_DISPLAY = "'Outfit', 'Space Grotesk', system-ui, sans-serif"
FONT_BODY = "'Source Sans 3', 'Space Grotesk', system-ui, sans-serif"
FONT_MONO = "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace"


def esc(t):
    return html.escape(t, quote=False)


def box(x, y, w, h, accent, title, subs, fs=9.4, lh=15.6, mono=True, bright=False):
    n = NODE[accent]
    t_font = FONT_DISPLAY if not mono else FONT_MONO
    s = [
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="5" fill="{PANEL}"/>',
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="5" fill="{n["fill"]}" '
        f'stroke="{n["stroke"]}" stroke-opacity="{0.72 if bright else 0.32}" stroke-width="{1.8 if bright else 1.2}"/>',
        f'<text x="{x + 16}" y="{y + 25}" fill="{n["title"]}" font-family="{t_font}" font-size="12" '
        f'font-weight="600" letter-spacing="0.2">{esc(title)}</text>',
    ]
    yy = y + 47
    for sub in subs:
        s.append(f'<text x="{x + 16}" y="{yy}" fill="{INK_SOFT}" font-family="{FONT_BODY}" '
                 f'font-size="{fs}">{esc(sub)}</text>')
        yy += lh
    return "<g>" + "".join(s) + "</g>"


def bnd(x, y, w, h, accent, dash="8 6", opacity=0.30):
    c = BAND[accent]
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="none" stroke="{c}" '
            f'stroke-opacity="{opacity}" stroke-width="1.1" stroke-dasharray="{dash}"/>')


def edge(d, color=FLOW["slate"], dash=None, w=1.5, arrow=True):
    da = f' stroke-dasharray="{dash}"' if dash else ""
    mk = f' marker-end="url(#{MARKER[color]})"' if arrow else ""
    return (f'<path d="{d}" fill="none" stroke="{color}" stroke-width="{w}" '
            f'stroke-linecap="round"{da}{mk}/>')


def lab(x, y, t, fs=8.4, fill=INK_SOFT, anchor="middle", fw="400", ls=None, family=FONT_BODY, italic=False):
    l = f' letter-spacing="{ls}"' if ls else ""
    i = ' font-style="italic"' if italic else ""
    return (f'<text x="{x}" y="{y}" fill="{fill}" font-family="{family}" font-size="{fs}" '
            f'text-anchor="{anchor}" font-weight="{fw}"{l}{i}>{esc(t)}</text>')


def badge(x, y, w, h, accent, title, kicker=None):
    """Node-style badge used by the publish-order chain."""
    n = NODE[accent]
    s = [
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{PANEL}"/>',
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{n["fill"]}" '
        f'stroke="{n["stroke"]}" stroke-opacity="0.30" stroke-width="1"/>',
    ]
    ty = y + (h / 2 + 4)
    if kicker:
        s.append(lab(x + 14, ty - 10, kicker, fs=8.6, fill=n["title"], anchor="start",
                     fw="600", family=FONT_MONO, ls="1.1"))
        s.append(f'<text x="{x + 14}" y="{ty + 9}" fill="{INK_T}" font-family="{FONT_DISPLAY}" '
                 f'font-size="14.5" font-weight="600">{esc(title)}</text>')
    else:
        s.append(f'<text x="{x + 14}" y="{ty}" fill="{INK_T}" font-family="{FONT_MONO}" '
                 f'font-size="11.4" font-weight="500">{esc(title)}</text>')
    return "<g>" + "".join(s) + "</g>"


svg = []
markers = "".join(
    f'<marker id="ar-{c}" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="10" '
    f'markerHeight="10" orient="auto" markerUnits="strokeWidth">'
    f'<path d="M0,0 L9,5 L0,10 z" fill="{stroke}"/></marker>'
    for c, stroke in FLOW.items()
)
logo_cmds = (
    '<defs>'
    + markers
    + '<linearGradient id="sf-grad" x1="0%" y1="0%" x2="100%" y2="100%">'
      f'<stop offset="0%" stop-color="{EM300}"/><stop offset="50%" stop-color="{EM400}"/>'
      f'<stop offset="100%" stop-color="{EM600}"/></linearGradient>'
    + f'<pattern id="grid" width="38" height="38" patternUnits="userSpaceOnUse">'
      f'<path d="M 38 0 L 0 0 0 38" fill="none" stroke="{GRID_STROKE}" stroke-opacity="0.05" stroke-width="1"/>'
      f'</pattern>'
    + '<path id="sf-ring" fill="none" d="M0 -35 L26.5 -17.5 L31.5 0 L26.5 26.5 L0 38.5 L-26.5 26.5 '
      'L-31.5 0 L-26.5 -17.5 Z"/>'
    + '</defs>'
)
svg.append(logo_cmds)
svg.append(f'<rect width="1600" height="1660" fill="{BG}"/><rect width="1600" height="1660" fill="url(#grid)"/>')

# Soft brand glows (radial emerald), matching the site's own background treatment.
svg.append(f'<ellipse cx="60" cy="20" rx="520" ry="360" fill="{EM500}" opacity="0.05"/>')
svg.append(f'<ellipse cx="1560" cy="1160" rx="560" ry="380" fill="{EM400}" opacity="0.045"/>')

# ---- Brand lockup -----------------------------------------------------------
svg.append(
    '<g transform="translate(58 34) scale(0.60)">'
    f'<use href="#sf-ring" stroke="url(#sf-grad)" stroke-width="6" stroke-linejoin="miter"/>'
    '<g transform="translate(0, -35)">'
    '<path d="M0 0 L-4.5 -6 L-2.5 -3 L0 -5 L2.5 -3 L4.5 -6 Z" fill="url(#sf-grad)" stroke="#051e1a" stroke-width="0.5"/>'
    f'<circle cx="-1.5" cy="-4" r="0.75" fill="#051e1a"/><circle cx="1.5" cy="-4" r="0.75" fill="#051e1a"/></g>'
    '<path d="M0 -35 L0 -23.5" stroke="url(#sf-grad)" stroke-width="4.5"/>'
    f'<use href="#sf-ring" stroke="{EM300}" stroke-width="1" opacity="0.24" transform="scale(0.524)"/>'
    '<path d="M-9 0 L-3.5 0 M9 0 L3.5 0 M0 -9 L0 -3.5 M0 9 L0 3.5" stroke="#FFFFFF" stroke-width="1.25" stroke-linecap="round"/>'
    '<circle cx="0" cy="0" r="3" fill="none" stroke="#FFFFFF" stroke-width="1.25"/>'
    f'<circle cx="0" cy="0" r="1.5" fill="{EM300}"/>'
    f'<path d="M-5.5 -5.5 L-7 -5.5 L-7 -7 M5.5 -5.5 L7 -5.5 L7 -7 M5.5 5.5 L7 5.5 L7 7 M-5.5 5.5 L-7 5.5 L-7 7" '
    f'stroke="{EM300}" stroke-width="1" stroke-linecap="round"/>'
    '</g>'
)
svg.append(
    f'<text x="126" y="54" font-family="{FONT_DISPLAY}" font-size="21" font-weight="600" letter-spacing="-0.3">'
    f'<tspan fill="{INK_T}">Solver</tspan><tspan fill="{EM300}">Forge</tspan></text>'
)
svg.append(f'<circle cx="268" cy="48" r="3.4" fill="{EM400}">'
           '<animate attributeName="opacity" values="1;0.35;1" dur="2.6s" repeatCount="indefinite"/></circle>')
svg.append(lab(282, 52, "system wiring · at a glance", fs=11.5, fill=INK_MUTED, anchor="start",
               family=FONT_MONO, ls="0.6"))
svg.append(lab(58, 84, "workspace v0.19.8 · 9 published crates · facade-first · zero-erasure Rust ·"
               " kernel repo /srv/lab/dev/solverforge/solverforge",
               fs=10.5, fill=INK_MUTED, anchor="start"))

# ---- Band 1: your code --------------------------------------------------------
svg.append(bnd(40, 104, 1520, 186, CY))
svg.append(lab(58, 127, "01 · YOUR CODE", fs=11, fill=NODE[CY]["title"], anchor="start", fw="700",
               family=FONT_DISPLAY, ls="2.6"))
svg.append(lab(180, 127, "model · configuration · entry point", fs=10, fill=INK_MUTED, anchor="start",
               family=FONT_MONO))
svg.append(box(80, 148, 420, 116, CY, "Your domain model · src/domain/mod.rs", [
    "planning_model! manifest — entity · fact · solution modules",
    "#[planning_solution] · #[planning_entity] · #[problem_fact]",
    "scalar & list variables · #[planning_pin] · list domain profiles"]))
svg.append(box(540, 148, 420, 116, AM, "solver.toml / solver.yaml", [
    "phases · move selectors · acceptors · foragers",
    "termination limits · environment · threads",
    "loaded into SolverConfig (solverforge-config)"]))
svg.append(box(1000, 148, 520, 116, CY, "Your app — main.rs", [
    "use solverforge::prelude::*;",
    "SolverManager::new(problem).solve()",
    "snapshots · events · analyze()"]))
svg.append(edge("M 280 332 V 290", color=FLOW["slate"]))
svg.append(lab(292, 312, "planning_model! expands the model", anchor="start", fs=8, fill=INK_SOFT))
svg.append(edge("M 750 290 V 332", color=FLOW["slate"]))
svg.append(lab(762, 314, "reads SolverConfig", anchor="start", fs=8, fill=INK_SOFT))
svg.append(edge("M 1100 290 V 332", color=FLOW["slate"]))
svg.append(lab(1112, 306, "single dependency", anchor="start", fs=8, fill=INK_SOFT))

# ---- Band 2: crate graph --------------------------------------------------------
svg.append(bnd(40, 346, 1520, 420, VI))
svg.append(lab(58, 369, "02 · CRATE GRAPH", fs=11, fill=NODE[VI]["title"], anchor="start", fw="700",
               family=FONT_DISPLAY, ls="2.6"))
svg.append(lab(230, 369, "9 published crates · one facade", fs=10, fill=INK_MUTED, anchor="start",
               family=FONT_MONO))
svg.append(box(90, 382, 440, 108, CY, "solverforge-macros", [
    "planning_model! · #[planning_*] attributes",
    "#[solverforge_constraints] — grouped node sharing",
    "proc-macro only — no runtime deps · emits ::solverforge::__internal::*"]))
svg.append(box(570, 382, 880, 108, VI, "solverforge — the facade", [
    "prelude · stream · planning · cvrp · __internal (doc-hidden) · console via feature",
    "re-exports macros + core + scoring + config + solver + bridge + cvrp",
    "apps & examples depend on this crate only · v0.19.8"], bright=True))
svg.append(edge("M 570 436 H 530", color=FLOW["violet"], arrow=False))
svg.append(edge("M 1010 490 V 500", color=FLOW["violet"], arrow=False))
svg.append(lab(1022, 505, "re-exports all sub-crates", anchor="start", fs=8, fill=INK_SOFT))
svg.append(bnd(60, 514, 1480, 236, SL, dash="6 5", opacity=0.22))
svg.append(lab(76, 536, "sub-crates — 7 of 9 published; arrow points at the dependency", anchor="start",
               fs=8, fill=INK_MUTED, italic=True))
svg.append(box(90, 548, 240, 116, VI, "solverforge-core", [
    "score types · domain traits", "descriptors · logical IDs", "dynamic slot contracts"]))
svg.append(box(370, 548, 240, 116, VI, "solverforge-scoring", [
    "ConstraintFactory streams", "SERIO incremental state", "Director · ScoreDirector"]))
svg.append(box(650, 548, 240, 190, EM, "solverforge-solver", [
    "runtime compiler — frozen graph", "phases · moves · selectors", "acceptors · foragers",
    "termination · completion gate", "SolverManager · realtime", "telemetry · candidate traces",
    "element-key validation"]))
svg.append(box(930, 548, 240, 116, EM, "solverforge-cvrp", [
    "VrpSolution · ProblemData", "distance meters", "route & savings hooks"]))
svg.append(box(90, 664, 240, 74, SL, "solverforge-console", [
    "tracing layer · banner", "phase & progress output"]))
svg.append(box(370, 664, 240, 74, VI, "solverforge-config", [
    "SolverConfig · TOML / YAML", "phases · selectors · termination"]))
svg.append(box(930, 664, 240, 74, RO, "solverforge-bridge", [
    "dynamic slots · DynamicScore", "host runner contract"]))
svg.append(edge("M 370 601 H 330", color=FLOW["violet"]))
svg.append(edge("M 650 601 H 610", color=FLOW["emerald"]))
svg.append(edge("M 650 704 H 610", color=FLOW["emerald"]))
svg.append(edge("M 930 601 H 890", color=FLOW["emerald"]))
svg.append(lab(910, 597, "meters", fs=8, fill=INK_SOFT))
svg.append(edge("M 930 704 H 890", color=FLOW["rose"]))
svg.append(lab(910, 700, "drives", fs=8, fill=INK_SOFT))
svg.append(edge("M 770 548 V 524 H 210 V 548", color=FLOW["violet"]))
svg.append(lab(490, 532, "descriptors", fs=8, fill=INK_SOFT))
# Publish order rail — reads as a numbered chain, matching the brand's numbered workflow.
svg.append(bnd(1210, 548, 290, 196, VI, dash="6 5", opacity=0.22))
svg.append(lab(1226, 572, "PUBLISH ORDER — CRATES.IO", fs=9, fill=NODE[VI]["title"], anchor="start",
               fw="700", family=FONT_MONO, ls="1.2"))
svg.append(lab(1226, 588, "staggered: core first, facade last", fs=7.8, fill=INK_MUTED, anchor="start"))
_pub = ["core", "macros", "scoring", "config", "solver", "bridge", "cvrp", "console", "facade"]
for i, name in enumerate(_pub):
    ry = 604 + i * 15.5
    svg.append(lab(1248, ry, f"{i + 1:02d}", fs=8.5, fill=EM400, anchor="start", fw="700", family=FONT_MONO))
    svg.append(lab(1272, ry, name, fs=9, fill=INK, anchor="start", family=FONT_MONO))
    if i < 8:
        svg.append(edge(f"M 1234 {ry + 4} V {ry + 11.5}", color=FLOW["violet"], w=1.1))

# ---- Band 3: solve pipeline -----------------------------------------------------
svg.append(bnd(40, 780, 1520, 150, EM))
svg.append(lab(58, 803, "03 · SOLVE PIPELINE", fs=11, fill=NODE[EM]["title"], anchor="start", fw="700",
               family=FONT_DISPLAY, ls="2.6"))
svg.append(lab(258, 803, "SolverManager::solve()", fs=10, fill=INK_MUTED, anchor="start", family=FONT_MONO))
svg.append(box(80, 814, 340, 102, EM, "1 · Build", [
    "SolverManager::new(problem)", "load solver.toml → SolverConfig",
    "compile RuntimeModel<…> — frozen solver graph"]))
svg.append(box(460, 814, 340, 102, EM, "2 · Construct", [
    "ConstructionHeuristic phase", "scalar groups · list element placement",
    "structural completion gate — every required slot assigned"]))
svg.append(box(840, 814, 340, 102, EM, "3 · Search", [
    "LocalSearch — moves · selectors · acceptors",
    "VND · PartitionedSearch (rayon) · Custom (typed)",
    "incremental SERIO delta scoring per move"]))
svg.append(box(1220, 814, 300, 102, EM, "4 · Complete", [
    "termination reached", "best solution published",
    "SolverStatus · snapshot · telemetry"]))
for x1, x2 in ((420, 460), (800, 840), (1180, 1220)):
    svg.append(edge(f"M {x1} 865 H {x2}", color=FLOW["emerald"]))
svg.append(edge("M 1370 916 V 970", color=FLOW["emerald"], arrow=False))
svg.append(lab(1382, 926, "retained job lifecycle", anchor="start", fill=NODE[EM]["title"], fs=8.6))
svg.append(lab(1382, 941, "Solving · Paused · Completed · Cancelled · Failed", anchor="start",
               fs=8, fill=INK_SOFT))

# ---- Band 4: runtime anatomy ----------------------------------------------------
svg.append(bnd(40, 946, 1520, 352, RO))
svg.append(lab(58, 969, "04 · INSIDE A RUN", fs=11, fill=NODE[RO]["title"], anchor="start", fw="700",
               family=FONT_DISPLAY, ls="2.6"))
svg.append(lab(228, 969, "the zero-erasure path", fs=10, fill=INK_MUTED, anchor="start", family=FONT_MONO))
svg.append(box(80, 980, 660, 178, VI, "constraint streams · solverforge-scoring", [
    "ConstraintFactory::new().for_each(model source)          .filter(…) .join(…)",
    "uni · bi · tri · quad · penta · cross · flattened · projected · exists",
    "grouped · complemented — collectors own extracted values",
    "terminal .penalize(weight).named(\"…\") / .reward(…).named(\"…\")",
    "compiled into IncrementalConstraint nodes by #[solverforge_constraints]",
    "shared grouped node owns retained state once — terminals stay independent",
    "score: HardSoftScore · HardMediumSoftScore · SoftScore · BendableScore · Decimal"]))
svg.append(box(800, 980, 340, 178, EM, "search phases · solverforge-solver", [
    "ConstructionHeuristic · LocalSearch · VND",
    "PartitionedSearch — rayon work split",
    "ExhaustiveSearch · CustomSearchPhase",
    "phases ordered by SolverConfig"]))
svg.append(box(1160, 980, 360, 178, EM, "moves · selectors · acceptors · foragers", [
    "change · swap · pillar · list change/swap/permute/reverse",
    "sublist · ruin/recreate · KOpt · compound · composite",
    "neighborhoods: list · nearby · sublist · cartesian · limited · union",
    "tabu / late acceptance / simulated annealing",
    "foragers: accepted count · best score · first best"]))
svg.append(edge("M 740 1069 H 800", color=FLOW["slate"]))
svg.append(edge("M 1140 1069 H 1160", color=FLOW["slate"]))
svg.append(edge("M 970 980 V 958 H 410", color=FLOW["rose"], dash="4 4"))
svg.append(edge("M 410 958 V 969", color=FLOW["rose"], dash="4 4"))
svg.append(lab(690, 948, "move selection loop", anchor="middle", fill=NODE[RO]["title"], fs=8.2, family=FONT_MONO))
svg.append(edge("M 970 1158 V 1168 H 410 V 1158", color=FLOW["rose"], dash="4 4", arrow=False))
svg.append(edge("M 410 1168 V 1184", color=FLOW["rose"], dash="4 4"))
svg.append(lab(700, 1162, "each applied move → incremental delta update — no full rescore", fill=NODE[RO]["title"], fs=8.4))
svg.append(box(80, 1184, 660, 96, AM, "director & delta scoring · zero-erasure hot path", [
    "&dyn is allowed only at declared boundaries (scorer arg, dynamic slots, problem change)",
    "moves are never cloned — cursors own candidates, the winner is taken by ownership"]))
svg.append(box(800, 1184, 720, 96, SL, "guarantees enforced at compile time", [
    "monomorphized generics end-to-end · no Box<dyn> · no Arc/Rc · no clone in move eval",
    "deterministic selector seeds from SolverConfig.random_seed · canonical enumeration order"]))

# ---- Band 5: consumers ------------------------------------------------------------
svg.append(bnd(40, 1322, 1520, 208, AM))
svg.append(lab(58, 1345, "05 · CONSUMERS & ECOSYSTEM", fs=11, fill=NODE[AM]["title"], anchor="start",
               fw="700", family=FONT_DISPLAY, ls="2.6"))
svg.append(lab(340, 1345, "the repo is the kernel; everything else builds on it", fs=10, fill=INK_MUTED,
               anchor="start", family=FONT_MONO))
svg.append(box(80, 1356, 460, 148, AM, "in-repo · examples/*", [
    "scalar-graph-coloring · nqueens — scalar assignment",
    "minimal-shift-scheduling — ScalarGroup + consecutive_runs",
    "list-tsp — list routing · mixed-job-shop — scalar + list",
    "",
    "cargo run -p scalar-graph-coloring"]))
svg.append(box(580, 1356, 460, 148, SL, "downstream repos · sibling checkouts", [
    "solverforge-cli — scaffolding & running models",
    "solverforge-py — Python bindings (pin released crates)",
    "solverforge-ui · -dash · -maps · -demoscene — visualization",
    "solverforge-bench · -gpt · -usecases — benchmarking & product surfaces"]))
svg.append(box(1080, 1356, 440, 148, SL, "host & output surfaces", [
    "crates.io — 9 crates, published in dependency order",
    "console feature — tracing layer for terminal output",
    "solverforge.org — site & docs",
    "GitHub Actions CI — make ci-local mirror"]))
for x in (540, 1040):
    svg.append(edge(f"M {x} 1430 H {x + 40}", color=FLOW["amber"]))

# ---- Legend -----------------------------------------------------------------------
svg.append(bnd(40, 1556, 1520, 92, SL, dash="6 5", opacity=0.18))
svg.append(lab(58, 1578, "LEGEND", fs=9, fill=INK_MUTED, anchor="start", fw="700", family=FONT_DISPLAY, ls="2"))
_lg = [
    (CY, "model / entry"),
    (VI, "core types + scoring"),
    (EM, "solver engine"),
    (AM, "config + host surface"),
    (RO, "runtime invariants"),
    (SL, "neutral / published"),
]
lx = 120
for accent, text in _lg:
    n = NODE[accent]
    svg.append(f'<rect x="{lx}" y="1568" width="15" height="13" rx="3" fill="{n["fill"]}" '
               f'stroke="{n["stroke"]}" stroke-opacity="0.5"/>')
    svg.append(lab(lx + 23, 1579, text, fs=8.8, fill=INK_SOFT, anchor="start"))
    lx += 210
svg.append(edge("M 1372 1574 H 1414", color=FLOW["emerald"], arrow=True))
svg.append(lab(1424, 1579, "dependency / flow", fs=8.8, fill=INK_SOFT, anchor="start"))
svg.append(edge("M 1372 1604 H 1414", color=FLOW["rose"], dash="4 4", arrow=True))
svg.append(lab(1424, 1609, "delta feedback · boundary", fs=8.8, fill=INK_SOFT, anchor="start"))
svg.append(lab(58, 1609, "Dashed frames = logical grouping only — every box is a module or crate in this repo,"
               " except sibling-checkout repos named above.", fs=8.2, fill=INK_MUTED, anchor="start"))

# ---- Assemble document --------------------------------------------------------------
doc = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>SolverForge — System Wiring At a Glance</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Outfit:wght@500;600;700;800&family=Source+Sans+3:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
  :root {{ color-scheme: dark; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; background: {BG}; color: {INK};
         font-family: {FONT_BODY}; }}
  .wrap {{ max-width: 1680px; margin: 0 auto; padding: 26px 28px 44px; }}
  .diagram {{ border: 1px solid rgba(110,231,183,0.14); border-radius: 9px; overflow: hidden;
              background: {BG}; box-shadow: 0 24px 60px rgba(3,31,26,0.55); }}
  svg {{ display: block; width: 100%; height: auto; }}
  .cards {{ display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin-top: 20px; }}
  .card {{ position: relative; border: 1px solid rgba(110,231,183,0.14); border-radius: 5px;
           padding: 17px 18px 16px; background: {PANEL}; overflow: hidden; }}
  .card::before {{ content: ""; position: absolute; inset: 0 0 auto 0; height: 2px;
                   background: linear-gradient(90deg, {EM300}, {EM600}); opacity: .85; }}
  .card-head {{ display: flex; align-items: baseline; gap: 9px; margin: 3px 0 11px; }}
  .card-kicker {{ font-family: {FONT_MONO}; font-size: 9px; letter-spacing: 1.3px;
                  color: {EM400}; font-weight: 700; }}
  .card-head h3 {{ font-family: {FONT_DISPLAY}; font-size: 13px; margin: 0; color: {INK_T};
                   font-weight: 600; letter-spacing: .2px; }}
  ul {{ margin: 0; padding: 0; list-style: none; }}
  li {{ font-size: 11px; line-height: 1.78; color: {INK_SOFT}; padding-left: 1.05em; position: relative; }}
  li::before {{ content: "▪"; position: absolute; left: 0; color: {EM400}; font-size: 8px; top: .28em; }}
  li b {{ color: {INK_T}; font-weight: 600; }}
  li code {{ font-family: {FONT_MONO}; font-size: 10px; color: {NODE[VI]['title']};
             background: rgba(167,139,250,0.10); padding: 0 4px; border-radius: 3px; }}
  footer {{ margin-top: 16px; font-family: {FONT_MONO}; font-size: 9.6px; color: {INK_MUTED}; padding: 0 4px; }}
  footer .sep {{ color: {EM600}; }}
</style>
</head>
<body>
<div class="wrap">
  <div class="diagram">
    <svg viewBox="0 0 1600 1660" xmlns="http://www.w3.org/2000/svg" role="img"
         aria-label="SolverForge architecture: your code, crate graph, solve pipeline, runtime anatomy, consumers">
      {"".join(svg)}
    </svg>
  </div>

  <div class="cards">
    <div class="card">
      <div class="card-head"><span class="card-kicker">01</span><h3>Crate Graph</h3></div>
      <ul>
        <li><b>solverforge</b> — the only dependency apps need: prelude, stream, planning, cvrp, hidden <code>__internal</code> for macro output.</li>
        <li><b>core</b> → score types, descriptors · <b>scoring</b> → streams, SERIO · <b>solver</b> → phases, moves, runtime.</li>
        <li><b>macros</b> is proc-macro-only; generated code resolves through the facade, so user crates compile.</li>
        <li>Publish order: core → macros → scoring → config → solver → bridge → cvrp → console → facade.</li>
      </ul>
    </div>
    <div class="card">
      <div class="card-head"><span class="card-kicker">02</span><h3>Solve Path</h3></div>
      <ul>
        <li>Your <b>planning_model!</b> manifest + <b>solver.toml</b> compile into an immutable runtime graph.</li>
        <li>Construction assigns every required scalar/list slot — the structural completion gate owns the boundary.</li>
        <li>Local search pulls moves from cursors; each applied move is an incremental delta to the retained constraint state.</li>
        <li>Termination → best solution / snapshot / telemetry; lifecycle: Solving → Paused → Completed | Cancelled | Failed.</li>
      </ul>
    </div>
    <div class="card">
      <div class="card-head"><span class="card-kicker">03</span><h3>Invariants & Consumers</h3></div>
      <ul>
        <li>Hot paths stay monomorphized: no <code>Box&lt;dyn&gt;</code>, no Arc/Rc, no cloning in move evaluation.</li>
        <li>Dynamic bridge is the declared host boundary — additive; the Rust path remains the performance ceiling.</li>
        <li>Downstream: solverforge-cli, -py, -ui, -dash, -maps, -bench live as sibling repos on the same contract.</li>
        <li>In-repo: five examples + <code>make build / test / lint / examples / ci-local</code>.</li>
      </ul>
    </div>
  </div>

  <footer>generated from the checked-in 0.19.8 workspace <span class="sep">·</span> crate manifests, wireframes, solver source
  <span class="sep">·</span> sibling-repo names denote external checkouts, not workspace members</footer>
</div>
</body>
</html>
"""

OUT.write_text(doc)

# Keep the README asset identical to the exported PNG when one is present.
synced = ""
if PNG.exists():
    PNG.parent.mkdir(parents=True, exist_ok=True)
    README_PNG.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(PNG, README_PNG)
    synced = f"; synced {README_PNG.relative_to(_REPO)}"

print(f"wrote {OUT} ({len(doc)} bytes){synced}")
if not PNG.exists():
    print(f"note: {PNG.relative_to(_REPO)} not present — render the .diagram element to PNG "
          "and re-run to sync the README asset")
