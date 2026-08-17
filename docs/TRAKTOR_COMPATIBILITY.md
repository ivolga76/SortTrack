# Traktor NML compatibility matrix

| SortTrack profile | Target | NML schema | Status |
|---|---|---:|---|
| `traktor_pro_3` | Traktor Pro 3.x | 19 | Golden fixture covered |
| `traktor_pro_4` | Traktor Pro 4.x | 19 | Golden fixture covered |

Both profiles currently use the same conservative NML 19 subset: collection entries, one playlist, Windows locations, metadata, tempo and primary keys. Profiles remain separate so a future Traktor change can receive a new serializer without silently changing older exports.

SortTrack always creates a separate `.nml` file. It never edits or replaces Traktor's `collection.nml`.
