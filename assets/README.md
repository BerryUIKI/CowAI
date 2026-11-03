# CowAI assets

Keep shared visual assets here so documentation and future application surfaces can reuse the same artwork.

## Brand files

| File | Role |
| --- | --- |
| [`brand/cowai-icon.svg`](brand/cowai-icon.svg) | Canonical, editable icon. Hand-drawn cubic Bézier paths and geometric shapes, a transparent canvas, and a `1000 × 1000` viewBox. |
| [`brand/cowai-icon.png`](brand/cowai-icon.png) | Derived `512 × 512` documentation image on a warm light background (`#f4f3ee`), keeping the dark outline visible in light and dark themes. |

The rounded cow head, eye patch, and two inspiration strokes form the icon. The artwork uses near-black (`#272622`) and white. Keep its proportions and clear space; the cow's white interior is intentional, while the SVG's exterior is transparent.

## Updating assets

1. Edit the canonical SVG directly. Keep meaningful element IDs and the accessible title/description. Use vector paths and shapes; do not embed raster images, fonts, scripts, or external resources.
2. Regenerate the PNG from the SVG rather than editing the export. With ImageMagick 7 and SVG support:

   ```bash
   magick -background "#f4f3ee" assets/brand/cowai-icon.svg -resize 512x512 -depth 8 assets/brand/cowai-icon.png
   ```

3. Check the icon at small sizes and preview the README in light and dark themes. Keep the source, derived export, and references together in the same small PR targeting `dev`.

Use stable, lowercase, kebab-case filenames. Git history records revisions; add a descriptive suffix only for a genuinely different asset variant. Link assets with relative repository paths so branch and release previews work without remote hosting.

Exploratory drafts, tracing experiments, and temporary previews belong in the ignored root `output/` or `tmp/` directories. Commit only maintained sources and the exports consumed by the project. These small files use ordinary Git; no asset pipeline or Git LFS is required for the current icon set.
