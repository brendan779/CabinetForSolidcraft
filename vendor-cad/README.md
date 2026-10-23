# Supplier CAD (local only)

Put Blum and Häfele CAD downloads (STEP preferred) here, one folder per supplier:

```
vendor-cad/blum/<article-number>.step
vendor-cad/hafele/<article-number>.step
```

**Nothing in this folder is committed.** The repository is public, and supplier CAD comes with
the supplier's own terms of use: we may use it, but we may not republish it. The app reads this
folder at run time and falls back to its own parametric stand-in models for anything missing.
