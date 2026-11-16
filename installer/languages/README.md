# Inno Setup translations

The installer and all five wizard languages are verified with Inno Setup 6.7.4.

The installer vendors the Belarusian, Kazakh, and Simplified Chinese message
files from the Inno Setup source tree at tag `is-6_7_3`:

- `Belarusian.isl`: `Files/Languages/Unofficial/Belarusian.isl`
- `Kazakh.islu`: `Files/Languages/Unofficial/Kazakh.islu`
- `ChineseSimplified.isl`: `Files/Languages/Unofficial/ChineseSimplified.isl`

Source: https://github.com/jrsoftware/issrc/tree/is-6_7_3/Files/Languages/Unofficial

The files are stored as UTF-8 without a byte-order mark and are marked as
project adaptations. Obsolete Inno Setup keys were removed, and legacy Kazakh
font overrides were dropped so the wizard inherits the current Segoe UI
defaults. The installer loads `Default.isl` first so messages added after a
translation's stated Inno Setup version have an English fallback instead of
becoming undefined.

The translations and Inno Setup are redistributed under the terms in
`LICENSE.txt`. Translation author credits are retained in each message file.
