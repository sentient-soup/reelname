// Frontend type definitions (replaces Drizzle-inferred types)

export interface Group {
  id: number;
  status: string;
  mediaType: string;
  folderPath: string;
  folderName: string;
  totalFileCount: number;
  totalFileSize: number;
  parsedTitle: string | null;
  parsedYear: number | null;
  tmdbId: number | null;
  tmdbTitle: string | null;
  tmdbYear: number | null;
  tmdbPosterPath: string | null;
  matchConfidence: number | null;
  destinationId: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface Job {
  id: number;
  groupId: number | null;
  status: string;
  mediaType: string;
  fileCategory: string;
  extraType: string | null;
  sourcePath: string;
  fileName: string;
  fileSize: number;
  fileExtension: string;
  parsedTitle: string | null;
  parsedYear: number | null;
  parsedSeason: number | null;
  parsedEpisode: number | null;
  parsedQuality: string | null;
  parsedCodec: string | null;
  tmdbId: number | null;
  tmdbTitle: string | null;
  tmdbYear: number | null;
  tmdbPosterPath: string | null;
  tmdbEpisodeTitle: string | null;
  matchConfidence: number | null;
  destinationId: number | null;
  destinationPath: string | null;
  transferProgress: number | null;
  transferError: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MatchCandidate {
  id: number;
  jobId: number | null;
  groupId: number | null;
  tmdbId: number;
  mediaType: string;
  title: string;
  year: number | null;
  posterPath: string | null;
  overview: string | null;
  confidence: number;
}

export interface Destination {
  id: number;
  name: string;
  type: string;
  basePath: string;
  sshHost: string | null;
  sshPort: number | null;
  sshUser: string | null;
  sshKeyPath: string | null;
  sshKeyPassphrase: string | null;
  movieTemplate: string | null;
  tvTemplate: string | null;
}
