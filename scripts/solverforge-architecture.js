#!/usr/bin/env node
'use strict';

// Compose the documentation page from the editable vector wiring map.
// Run: node scripts/solverforge-architecture.js [repository-root]
const fs = require('node:fs');
const path = require('node:path');
const root = process.argv[2] ? path.resolve(process.argv[2]) : path.resolve(__dirname, '..');
const version = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8')
  .match(/\[workspace\.package\][\s\S]*?version = "([^"]+)"/)[1];
const source = fs.readFileSync(path.join(root, 'assets/solverforge-architecture.svg'), 'utf8');
const escape = s => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');

// Keep the existing node geometry, but use the site's light documentation tokens.
const colors = {
  '#051e1a': '#ffffff', '#083029': '#ffffff', '#8fb5a8': '#48655d',
  '#cfe9df': '#163830', '#ecfdf5': '#163830', '#5e8a7e': '#48655d',
  '#99f6e4': '#047857', '#c4b5fd': '#047857', '#a7f3d0': '#047857',
  '#fde68a': '#715709', '#fda4af': '#715709', '#c6e2d8': '#163830',
  '#a78bfa': '#059669', '#5eead4': '#059669', '#6ee7b7': '#10b981',
  '#34d399': '#059669', '#fb7185': '#b58b08', '#fcd34d': '#b58b08',
  '#6ea99b': '#48655d',
};
let light = source.replace(/#[0-9a-f]{6}/gi, c => colors[c.toLowerCase()] || c);
light = light.replace(/rgba\(167,139,250,0\.10\)|rgba\(20,184,166,0\.10\)|rgba\(16,185,129,0\.12\)/g, 'rgba(16,185,129,0.07)')
  .replace(/rgba\(251,113,133,0\.10\)/g, 'rgba(234,179,8,0.07)')
  .replace(/rgba\(110,169,155,0\.08\)/g, 'rgba(6,47,40,0.03)')
  .replace(/<ellipse[^>]*\/>/g, '')
  .replace(/<rect[^>]*stroke-dasharray[^>]*\/>/g, '')
  .replace(/<text[^>]*>0[1-5] ·[^<]*<\/text>/g, '')
  .replace(/<text[^>]*>LEGEND<\/text>/g, '')
  .replace(/<text[^>]*>model · configuration · entry point<\/text>/g, '')
  .replace(/<text[^>]*>9 published crates · one facade<\/text>/g, '')
  .replace(/<text[^>]*>SolverManager::solve\(\)<\/text>/g, '')
  .replace(/<text[^>]*>the zero-erasure path<\/text>/g, '')
  .replace(/<text[^>]*>the repo is the kernel; everything else builds on it<\/text>/g, '')
  .replace(/kernel repo \/srv\/lab\/dev\/solverforge\/solverforge/g, 'native Rust planning engine');

// SVG fragment identifiers are local to each panel, including referenced markers.
function diagram(svg, id, viewBox, label) {
  const [, top, , height] = viewBox.split(' ').map(Number);
  // Omit off-panel labels rather than letting glyphs bleed across a crop edge.
  svg = svg.replace(/<text\b([^>]*)>[\s\S]*?<\/text>/g, (text, attributes) => {
    const y = Number(attributes.match(/\by="([^"]+)"/)?.[1]);
    return y < top + 8 || y > top + height - 3 ? '' : text;
  });
  return svg.replace(/id="([^"]+)"/g, (_, name) => `id="${id}-${name}"`)
    .replace(/url\(#([^)]+)\)/g, (_, name) => `url(#${id}-${name})`)
    .replace(/href="#([^"]+)"/g, (_, name) => `href="#${id}-${name}"`)
    .replace(/viewBox="0 0 1600 1660"/, `viewBox="${viewBox}"`)
    .replace(/aria-label="[^"]*"/, `aria-label="${escape(label)}"`);
}
const panels = [
  ['model', '01', 'Model and configuration', 'Entities, facts, variables, and configuration enter through the public facade.', '60 142 1480 190'],
  ['crates', '02', 'Crate dependencies', 'Every direct workspace dependency is shown, pointing to the dependency. The dashed facade-to-console edge is feature-gated.', '60 378 1480 372'],
  ['pipeline', '03', 'Solve pipeline', 'Build the runtime, complete required assignments, run eligible search, and retain the result. Default search requires effective solver termination; configured limits remain binding during construction.', '60 810 1480 135'],
  ['runtime', '04', 'Runtime', 'Selectors own candidates. Scoring retains incremental state. Only the selected move transfers by ownership.', '60 952 1480 335'],
  ['ecosystem', '05', 'Examples and integrations', 'Workspace examples use the same facade. CLI, bindings, and visualization tools live in separate repositories.', '60 1350 1480 160'],
];
const nav = panels.map(([id, n, title]) => `<a href="#${id}"><span>${n}</span>${title}</a>`).join('');
const sections = panels.map(([id, n, title, caption, viewBox]) => `
<section class="architecture-panel" id="${id}" aria-labelledby="${id}-title">
  <div class="panel-heading"><div><p class="eyebrow">${n} / ${id === 'crates' ? 'Crate graph' : id}</p><h2 id="${id}-title">${title}</h2></div><p>${caption}</p></div>
  <div class="diagram-scroll" tabindex="0" aria-label="${id} wiring diagram; scroll horizontally on small screens">${diagram(light, id, viewBox, caption)}</div>
</section>`).join('');
const brand = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 248 50\">\n  <defs>\n    <linearGradient id=\"grad-dark-clean\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"100%\">\n      <stop offset=\"0%\" style=\"stop-color:#6ee7b7;stop-opacity:1\" />\n      <stop offset=\"50%\" style=\"stop-color:#34d399;stop-opacity:1\" />\n      <stop offset=\"100%\" style=\"stop-color:#059669;stop-opacity:1\" />\n    </linearGradient>\n  </defs>\n  <g>\n    <g transform=\"translate(25, 25) scale(0.52)\">\n      <path d=\"M0 -35 L26.5 -17.5 L31.5 0 L26.5 26.5 L0 38.5 L-26.5 26.5 L-31.5 0 L-26.5 -17.5 Z\"\n        stroke=\"url(#grad-dark-clean)\" stroke-width=\"6\" fill=\"none\" stroke-linejoin=\"miter\"/>\n      <g transform=\"translate(0, -35)\">\n        <path d=\"M0 0 L-4.5 -6 L-2.5 -3 L0 -5 L2.5 -3 L4.5 -6 Z\"\n          fill=\"url(#grad-dark-clean)\" stroke=\"#000000\" stroke-width=\"0.5\"/>\n        <circle cx=\"-1.5\" cy=\"-4\" r=\"0.75\" fill=\"#000000\"/>\n        <circle cx=\"1.5\" cy=\"-4\" r=\"0.75\" fill=\"#000000\"/>\n      </g>\n      <path d=\"M0 -35 L0 -23.5\" stroke=\"url(#grad-dark-clean)\" stroke-width=\"4.5\"/>\n      <path d=\"M0 -17.5 L14 -9 L16.5 0 L14 14 L0 21 L-14 14 L-16.5 0 L-14 -9 Z\"\n        stroke=\"#6ee7b7\" stroke-width=\"1\" fill=\"none\" opacity=\"0.24\"/>\n      <circle cx=\"0\" cy=\"-35\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"26.5\" cy=\"-17.5\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"31.5\" cy=\"0\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"26.5\" cy=\"26.5\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"0\" cy=\"38.5\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"-26.5\" cy=\"26.5\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"-31.5\" cy=\"0\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <circle cx=\"-26.5\" cy=\"-17.5\" r=\"2.5\" fill=\"#6ee7b7\" opacity=\"0.18\"/>\n      <path d=\"M-9 0 L-3.5 0 M9 0 L3.5 0 M0 -9 L0 -3.5 M0 9 L0 3.5\"\n        stroke=\"#FFFFFF\" stroke-width=\"1.25\" stroke-linecap=\"round\"/>\n      <circle cx=\"0\" cy=\"0\" r=\"3\" fill=\"none\" stroke=\"#FFFFFF\" stroke-width=\"1.25\"/>\n      <circle cx=\"0\" cy=\"0\" r=\"1.5\" fill=\"#6ee7b7\"/>\n      <path d=\"M-5.5 -5.5 L-7 -5.5 L-7 -7 M5.5 -5.5 L7 -5.5 L7 -7 M5.5 5.5 L7 5.5 L7 7 M-5.5 5.5 L-7 5.5 L-7 7\"\n        stroke=\"#6ee7b7\" stroke-width=\"1\" stroke-linecap=\"round\"/>\n    </g>\n    <text x=\"58\" y=\"32\" font-family=\"Avenir Next, Avenir, Helvetica Neue, Helvetica, sans-serif\" font-size=\"24\" font-weight=\"400\" letter-spacing=\"-0.02em\">\n      <tspan fill=\"#ecfdf5\">Solver</tspan><tspan fill=\"#6ee7b7\">Forge</tspan>\n    </text>\n  </g>\n</svg>";
const html = `<!doctype html>
<!-- Generated by scripts/solverforge-architecture.js; edit the script or vector source. -->
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Architecture | SolverForge</title>
<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Outfit:wght@500;600;700;800&family=Source+Sans+3:wght@400;500;600;700&display=swap">
<style>
:root{--sf-bg:#f7fbf8;--sf-panel:#fff;--sf-panel-soft:#eef7f1;--sf-panel-dark:#062f28;--sf-panel-dark-soft:#0d473c;--sf-border:rgba(6,47,40,.12);--sf-text:#163830;--sf-text-soft:#48655d;--sf-text-invert:#ecfdf5;--sf-accent:#10b981;--sf-accent-strong:#059669;--sf-gold:#eab308;--sf-radius:5px;--sf-shadow:0 24px 60px rgba(6,47,40,.12);--sf-width:min(1180px,calc(100% - 2rem))}
*{box-sizing:border-box}html{scroll-behavior:smooth;scroll-padding-top:7rem}body{margin:0;font-family:'Source Sans 3',system-ui,sans-serif;color:var(--sf-text);background:radial-gradient(circle at top left,rgba(16,185,129,.09),transparent 28rem),linear-gradient(180deg,#fbfefc,#f3faf6);line-height:1.65}
a{color:var(--sf-accent-strong);text-underline-offset:.18em}a:hover{color:#047857}h1,h2,h3{font-family:Outfit,system-ui,sans-serif;line-height:1.05;letter-spacing:-.04em;margin:0 0 1rem}h1{font-size:clamp(2.8rem,5vw,4.6rem)}h2{font-size:1.4rem}p{margin:0}svg{display:block;width:100%;height:auto}code{font-family:'JetBrains Mono',monospace;font-size:.85em}
.site-header{position:sticky;top:0;z-index:20;backdrop-filter:blur(18px);background:rgba(6,47,40,.94);border-bottom:1px solid rgba(236,253,245,.08)}.header-inner{width:var(--sf-width);margin:auto;min-height:5.5rem;display:flex;align-items:center;justify-content:space-between;gap:1rem}.brand{width:210px;flex-shrink:0}.header-links{display:flex;gap:1.5rem;align-items:center}.header-links a{color:rgba(236,253,245,.9);font-weight:600;text-decoration:none}.header-links .active{background:rgba(16,185,129,.16);padding:.65rem .85rem;border-radius:5px;color:#fff}
main{width:var(--sf-width);margin:auto;padding:1.5rem 0 2rem}.page-heading{margin-bottom:1rem}.page-heading h1{font-size:2rem;margin-bottom:.4rem}.page-heading p,.legend{color:var(--sf-text-soft)}.eyebrow{font-size:.75rem;color:var(--sf-accent-strong);margin:0 0 .4rem}
.section-nav{display:flex;flex-wrap:wrap;gap:.35rem;margin:1rem 0;padding:.6rem;background:rgba(6,47,40,.06);border-radius:5px}.section-nav a{flex:1;padding:.6rem .65rem;color:var(--sf-text-soft);font-size:.95rem;font-weight:600;text-decoration:none;border-radius:5px;white-space:nowrap}.section-nav a:hover{background:#062f28;color:#ecfdf5}.section-nav span{color:#059669;margin-right:.45rem;font-family:'JetBrains Mono';font-size:.75rem}
.architecture-panel{margin:0 0 1.5rem;padding:1rem;border:1px solid var(--sf-border);border-radius:5px;background:var(--sf-panel);box-shadow:0 14px 36px rgba(6,47,40,.06)}.panel-heading{display:grid;grid-template-columns:1fr 1fr;gap:2rem;align-items:end;margin-bottom:1.4rem}.panel-heading h2{margin-bottom:0}.panel-heading>p{color:var(--sf-text-soft);font-size:1.05rem}.diagram-scroll{overflow-x:auto;border-top:1px solid var(--sf-border);padding-top:1rem}.diagram-scroll svg{min-width:1050px}.diagram-scroll:focus-visible{outline:2px solid var(--sf-accent);outline-offset:4px}
.legend{font-size:.9rem;margin-top:1rem}
@media(max-width:980px){.panel-heading{grid-template-columns:1fr;gap:.5rem}}
@media(max-width:760px){.header-inner{min-height:4rem;flex-wrap:wrap;padding:.6rem 0}.brand{width:150px}.header-links{gap:.8rem;font-size:.9rem}.header-links .active{display:none}.architecture-panel{padding:1rem}.section-nav a{flex-basis:45%}.diagram-scroll svg{min-width:1050px}}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}}
@media print{.site-header,.section-nav{display:none}.architecture-panel{break-inside:avoid;box-shadow:none}.diagram-scroll svg{min-width:0}main{width:100%;padding:0}}
</style></head><body>
<header class="site-header"><div class="header-inner"><a class="brand" href="https://solverforge.org" aria-label="SolverForge home">${brand}</a><nav class="header-links" aria-label="Site"><a href="https://solverforge.org/docs/">Documentation</a><a href="https://github.com/SolverForge/solverforge">Source</a><a class="active" href="#model">Architecture</a></nav></div></header>
<main><section class="page-heading" aria-labelledby="page-title"><h1 id="page-title">SolverForge architecture</h1><p>Workspace ${version} · Model, crate dependencies, solve pipeline, runtime, and integrations.</p></section>
<nav class="section-nav" aria-label="Diagram sections">${nav}</nav>${sections}
<p class="legend">Solid arrows indicate dependencies or control flow. Dashed arrows indicate the optional console dependency or runtime score feedback.</p>
</main>
</body></html>\n`;
const target = path.join(root, 'docs/architecture.html');
fs.writeFileSync(target, html);
console.log(`Generated ${path.relative(root, target)} (${Buffer.byteLength(html)} bytes)`);
