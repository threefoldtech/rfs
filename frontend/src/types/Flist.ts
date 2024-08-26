export interface Flist {
  auth: string;
  email: string;
  identity_token: string;
  image_name: string;
  password: string;
  registry_token: string;
  server_address: string;
  username: string;
}


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
