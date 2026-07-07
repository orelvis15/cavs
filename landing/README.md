# CAVS website

A multi-page static site for CAVS. No build step, no runtime dependencies —
plain HTML/CSS/JS with a shared design system. Dark/light theme (defaults to the
system preference), transparent logo, and a changelog loaded live from the repo.

## Pages

```
landing/
├── index.html          # Home — overview, headline benchmarks, product preview
├── benchmarks.html     # Full benchmark suite, documented, with charts
├── products.html       # Every product + the CLI command reference + quickstart
├── changelog.html      # Release history, fetched live from CHANGELOG.md
├── .nojekyll           # serve assets/ verbatim on GitHub Pages
├── assets/
│   ├── styles.css                # shared design system + dark/light themes
│   ├── app.js                    # nav, theme toggle, reveal, FAQ, copy
│   ├── charts.js                 # dependency-free SVG bar charts
│   ├── changelog.js              # live CHANGELOG.md fetch + markdown render
│   ├── logo-transparent-128.png  # transparent nav/footer logo + favicon
│   ├── logo-transparent-64.png   # small transparent logo (spare)
│   ├── logo-large.png            # transparent 1254² source logo
│   ├── logo.png / og-thumb.webp  # legacy favicon + social preview image
│   └── ...
└── README.md
```

## Theme

Dark is the base. Light kicks in from `prefers-color-scheme` when the visitor
hasn't chosen, and the nav toggle sets an explicit `data-theme` that persists in
`localStorage`. Terminal and code blocks stay dark in both themes on purpose.

## Changelog

`assets/changelog.js` fetches `CHANGELOG.md` straight from the repository and
renders it in the browser, so the page is always current. The repo/branch are
constants at the top of that file (`orelvis15/cavs`, `main`); relative doc links
resolve to the GitHub blob view. If the fetch fails, it links out to GitHub.

## Charts

`assets/charts.js` reads `data-chart='{…}'` JSON from `.chart` elements and draws
grouped horizontal bar charts as inline SVG. Values are always printed, so short
bars over a huge dynamic range stay honest and readable. The bars are styled via
CSS variables, so they follow the active theme.

## Preview locally

```bash
cd landing
python3 -m http.server 8080
# open http://localhost:8080
```

The changelog needs network access to reach GitHub's raw endpoint.

## Publish on GitHub Pages

**Option A — `/docs` on the default branch (simplest)**

1. Copy this folder to `docs/` at the repo root.
2. Repo → Settings → Pages → Source: *Deploy from a branch* → branch `main`, folder `/docs`.

**Option B — GitHub Actions from `landing/`**

```yaml
name: Deploy site to Pages
on:
  push:
    branches: [main]
    paths: ["landing/**"]
permissions:
  pages: write
  id-token: write
concurrency:
  group: pages
  cancel-in-progress: true
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deploy.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/upload-pages-artifact@v3
        with:
          path: landing
      - id: deploy
        uses: actions/deploy-pages@v4
```

## Editing

Design tokens (colors, fonts, radii) are CSS variables at the top of
`assets/styles.css`, with a light-theme override block right below. Benchmark
numbers are plain HTML tables and `data-chart` JSON specs — edit them in place
when new releases land. All figures come from the measured benchmarks in
`docs/BENCHMARKS.md`. The site deliberately avoids version-vs-version CAVS
comparisons: features are presented as current capabilities.
