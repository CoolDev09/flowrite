# Offline English grammar data

`en_rules.bin` and `en_tokenizer.bin` are the unmodified English binaries from
nlprule **0.6.4**, validated using `nlprule-build` 0.6.4. Their source is
LanguageTool **5.2**. The binaries are licensed under **LGPL-2.1**, independently
of nlprule's MIT/Apache-2.0 Rust code. The full license is in `COPYING.txt`.

- Binary release: https://github.com/bminixhofer/nlprule/releases/tag/0.6.4
- nlprule source/build tools: https://github.com/bminixhofer/nlprule/tree/0.6.4
- LanguageTool source: https://github.com/languagetool-org/languagetool/tree/v5.2
- License source: https://github.com/languagetool-org/languagetool/blob/v5.2/COPYING.txt

The executable embeds these data files; no download occurs when flowrite runs.
To replace or modify them, use the compatible nlprule build tools, place the
resulting English binaries here, then rebuild flowrite. This repository includes
the application source and a locked dependency manifest so it can be rebuilt
with changed grammar data. Build-time downloading is opt-in via
`FLOWRITE_FETCH_GRAMMAR=1`; normal builds use locally prepared data. A fresh
checkout obtains the version-pinned release assets with
`python scripts/prepare_assets.py`; it verifies archive and file checksums.
