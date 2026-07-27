---
# Keep this comment and all scalar styles exactly.
title: "Résumé – Δ research 🧠"
aliases: [Mind Vault, "Second Brain"]
tags:
  - knowledge-management
  - "unicode/日本語"
type: concept
custom_nested:
  owner: Ada
  priority: 03
custom_folded: >-
  This folded scalar must remain
  in its original representation.
custom_literal: |
  first line
  second line
---

# Résumé – Δ research 🧠

> [!NOTE]+ Obsidian callout
> Preserve this extension without normalizing it.

A wikilink with an alias: [[Architecture/Canonical Storage|storage model]].
An embedded note: ![[Evidence/Source Note#Quoted passage]].
An embedded attachment: ![[assets/diagram 01.png]].

## Decision & evidence

The decision cites [a standard Markdown target](../Evidence/source.md#claim)
and a block reference [[Evidence/Source Note#^source-claim]].

This paragraph has a stable block identifier. ^decision-anchor

| Capability | State |
| :-- | --: |
| Exact retrieval | 1 |
| Semantic retrieval | 2 |

- [x] Preserve task state
- [ ] Preserve the unchecked item

Inline math stays `$E = mc^2$`.

$$
\int_0^1 x^2\,dx = \frac{1}{3}
$$

```dataview
TABLE file.mtime AS "Modified"
FROM #knowledge-management
WHERE custom_nested.owner = "Ada"
```

<details data-mindvault-unknown="preserve">
  <summary>Raw HTML</summary>
  <p>Do not reserialize this block.</p>
</details>

<!-- Preserve this comment, spacing, and punctuation. -->

Reference-style link: [MindVault contract][contract].

[contract]: ../../knowledge-workspace-document-contract.md "Document contract"

Footnote reference.[^source]

[^source]: Footnote text with `inline code` and [[Another Note]].
