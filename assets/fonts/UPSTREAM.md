# Bundled interface fonts

These are original, unmodified static fonts from the official Adobe and Noto
projects. They are embedded in GPUI; no font installation, runtime download or
system font package is required. The same files are used on Windows and Linux.

## Source Sans 3

- Official release: **3.052R** (version **3.052**), repository
  <https://github.com/adobe-fonts/source-sans>.
- Pinned commit: `ed1808970eb3c7301c9a523bee26473ba0bb62fa`.
- Original files: `OTF/SourceSans3-Regular.otf`,
  `OTF/SourceSans3-Medium.otf`, `OTF/SourceSans3-Semibold.otf`.
- [SIL OFL 1.1](source-sans-3/OFL.txt) is copied verbatim from the pinned
  commit's `LICENSE.md`, including copyright and Reserved Font Name notices.

## Noto Sans SC (Simplified Chinese)

- Official release: **Sans2.004**, repository
  <https://github.com/notofonts/noto-cjk>.
- Pinned commit: `523d033d6cb47f4a80c58a35753646f5c3608a78`.
- Original files: `Sans/SubsetOTF/SC/NotoSansSC-Regular.otf` and
  `Sans/SubsetOTF/SC/NotoSansSC-Medium.otf`.
- This is the official Mainland Chinese regional subset of **Noto Sans CJK SC**,
  with upstream family name **Noto Sans SC**. We have not subset, renamed,
  converted or otherwise modified these files ourselves.
- [SIL OFL 1.1](noto-sans-sc/OFL.txt) is copied verbatim from the pinned commit's
  root `LICENSE`. The [copyright notice](noto-sans-sc/NOTICE.txt) also preserves
  the original fonts' name-table attribution to Adobe (2014-2021).

## Redistribution

Both licenses permit bundling and redistribution with software, including
commercial collections and operating-system distributions. Preserve copyright
notices and the complete OFL texts. Fonts must not be sold by themselves;
modified font versions must comply with Reserved Font Name conditions.

The application's MIT License does **not** relicense the fonts. Font recipients
retain their rights under OFL 1.1. All font notices are also included in
THIRD_PARTY_NOTICES.txt and in Windows/Linux packages under `licenses/fonts`.

[SHA256SUMS](SHA256SUMS) pins the exact font and notice bytes. Verification and
packaging reject mismatches; source builds use checked-in assets rather than a
moving download URL. Independent downloads use the corresponding official
repository's raw URL pinned to the commit and original path listed above.
