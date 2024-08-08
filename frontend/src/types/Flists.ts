export interface FlistBody {
  is_file: Boolean;
  last_modified: bigint;
  name: string;
  path_uri: string;
  progress: number;
}

export interface FlistsResponseInterface {
  [key: string]: FlistBody[];
}
