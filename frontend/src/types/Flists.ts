export interface FlistBody {
  is_file: Boolean;
  last_modified: bigint;
  name: string;
  path_uri: string;
  progress: number;
  size: number;
}

export interface FlistsResponseInterface {
  [key: string]: FlistBody[];
}

export interface FlistPreview{
  checksum: string;
  content: string[];
  metadata: string;
}

export interface FlistPreviewRequest{
  flist_path: string
}