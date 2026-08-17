export const GENRE_TAXONOMY = {
  Breaks: [],
  "Drum & Bass": [],
  House: [],
  Techno: [
    "Melodic Techno",
    "Hard Techno",
    "Percussive Techno",
    "Peak Time Techno",
    "Acid Techno",
    "Raw Techno",
    "Schranz",
  ],
  Trance: [],
  Hardcore: [],
  Hardstyle: [],
  Electropunk: [],
} as const;

const aliases: Record<string, string> = {
  breacks: "Breaks",
  break: "Breaks",
  breaks: "Breaks",
  dnb: "Drum & Bass",
  "d&b": "Drum & Bass",
  "drum'n'bass": "Drum & Bass",
  "drum and bass": "Drum & Bass",
  "drum & bass": "Drum & Bass",
  "pictime techno": "Peak Time Techno",
  "peak time techno": "Peak Time Techno",
  "percussion techno": "Percussive Techno",
  "percussive techno": "Percussive Techno",
  electropunk: "Electropunk",
  "electro punk": "Electropunk",
};

for (const [genre, subgenres] of Object.entries(GENRE_TAXONOMY)) {
  aliases[genre.toLowerCase()] = genre;
  for (const subgenre of subgenres) aliases[subgenre.toLowerCase()] = subgenre;
}

export interface NormalizedGenre {
  original: string;
  genre: string | null;
  subgenre: string | null;
}

export function normalizeGenre(value: string): NormalizedGenre {
  const original = value.trim();
  const normalized = aliases[original.toLowerCase()] ?? null;
  if (!normalized) return { original, genre: null, subgenre: null };

  for (const [genre, subgenres] of Object.entries(GENRE_TAXONOMY)) {
    if (genre === normalized) return { original, genre, subgenre: null };
    if ((subgenres as readonly string[]).includes(normalized)) {
      return { original, genre, subgenre: normalized };
    }
  }

  return { original, genre: null, subgenre: null };
}

export function genreLabel(genre: string | null, subgenre: string | null): string {
  if (!genre) return "Не классифицировано";
  return subgenre ? `${genre} → ${subgenre}` : genre;
}
