export interface FlistBody {
  isFile: Boolean;
  lastModified: bigint;
  name: string;
  pathUri: string;
  progress: number;
}

export interface FlistsResponseInterface {
  flists: Map<string, FlistBody[]>;
}
