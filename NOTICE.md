# Third-party assets

RustyCV embeds the following into its binary. Their licences are reproduced in
`assets/licenses/`.

## Fonts

| Family | Source | Licence |
| --- | --- | --- |
| Inter | [rsms/inter](https://github.com/rsms/inter) | SIL Open Font License 1.1 |
| IBM Plex Sans | [IBM/plex](https://github.com/IBM/plex) | SIL Open Font License 1.1 |
| IBM Plex Mono | [IBM/plex](https://github.com/IBM/plex) | SIL Open Font License 1.1 |
| Source Sans 3 | [adobe-fonts/source-sans](https://github.com/adobe-fonts/source-sans) | SIL Open Font License 1.1 |
| Source Serif 4 | [adobe-fonts/source-serif](https://github.com/adobe-fonts/source-serif) | SIL Open Font License 1.1 |

## Icons

Icons in `assets/icons/` are from
[Font Awesome Free 6](https://fontawesome.com), licensed **CC BY 4.0**.
The attribution comment Font Awesome ships inside each SVG is left intact.

## Rendering engine

PDFs are produced by [Typst](https://typst.app) (Apache-2.0), linked as a
library rather than invoked as a subprocess.

## A note on the `flowcv` template

The `flowcv` template reproduces the layout of FlowCV's default single-column
resume, measured from a published resume's rendered markup. It is an
independent reimplementation in Typst: no FlowCV code, stylesheet or asset is
copied or redistributed here. "FlowCV" is a trademark of its owners and is used
only to name the layout the template imitates.
