# Garage s3 server with flist

## Requirements

- tfcmd
- docker2fl
- rust
- docker
- git
- sqlite
- minio (or any third-party tool you want to use)
- caddy

### Install tfcmd

```bash
wget https://github.com/threefoldtech/tfgrid-sdk-go/releases/download/v0.15.11/tfgrid-sdk-go_Linux_x86_64.tar.gz
mkdir tfgrid-sdk-go
tar -xzf tfgrid-sdk-go_Linux_x86_64.tar.gz -C tfgrid-sdk-go
sudo mv tfgrid-sdk-go/tfcmd /usr/bin/
sudo rm -rf tfgrid-sdk-go_Linux_x86_64.tar.gz tfgrid-sdk-go
```

- Login to tfcmd

```bash
tfcmd login
```

### Install rust

```bash
apt-get update
apt-get install -y curl
curl https://sh.rustup.rs -sSf | sh
export PATH="$HOME/.cargo/bin:$PATH"
apt-get install -y build-essential
apt-get install -y musl-dev musl-tools
apt-get update
```

### Install docker

```bash
apt-get update
apt-get install -y ca-certificates curl
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
apt-get update
dockerd > docker.log 2>&1 &
```

### Install docker2fl

```bash
git clone https://github.com/threefoldtech/rfs.git
cd rfs
rustup target add x86_64-unknown-linux-musl
cargo build --features build-binary --release --target=x86_64-unknown-linux-musl
mv ./target/x86_64-unknown-linux-musl/release/docker2fl /usr/local/bin
```

### Install sqlite

```bash
apt update
apt install sqlite3 
```

### Install minio

```bash
curl https://dl.min.io/client/mc/release/linux-amd64/mc \
  --create-dirs \
  -o $HOME/minio-binaries/mc
chmod +x $HOME/minio-binaries/mc
export PATH=$PATH:$HOME/minio-binaries/
```

### Install Caddy

```bash
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy
```

## Usage

### Deploy garage server

Run garage server using garage server [script](./deploy_garage.sh)

```bash
chmod +x deploy_garage.sh
./deploy_garage.sh
```

This script includes:

1. Deploy a vm with mycelium IP to run garage s3 server over it.
2. Install garage in the vm.
3. Run the garage server with the given configuration.

### Manage buckets in garage server

Manage your buckets using manage buckets [script](./manage_buckets.sh)

```bash
export MYCELIUM_IP=<"your machine mycelium IP which has your garage server">
chmod +x manage_buckets.sh
./manage_buckets.sh
```

This script includes:

1. Create 2 buckets in garage server one for `flist` and the other for `blobs`.
2. Allow web for both buckets to be able to serve them.
3. Create 2 keys one for write and the other for read only. The `write-key` will be used to upload the flist and the blobs through rfs. The `read-key` should be updated for flist and blobs to prevent updating them.
4. Adding the keys with their permissions to the bucket.

> *NOTE:*  Don't forget to save your read and write keys (ID and secret).

### Convert docker images to flist and upload it

- Convert your image to an flist, The content will be uploaded over blobs buckets

```bash
export IMAGE=<"Your image for example `threefolddev/ubuntu:22.04`">
export WRITE_KEY_ID=<"your key ID">
export WRITE_KEY_SECRET=<"your key secret">
export MYCELIUM_IP=<"your machine mycelium IP which has your garage server">

docker2fl -i $IMAGE -s 's3://$WRITE_KEY_ID:$WRITE_KEY_SECRET@$[$MYCELIUM_IP]:3900/blobs?region=garage'
```

- Update the key to the read only key

```bash TODO:
sqlite3
.open "<your flist file name>"
update route set url="s3://<your read key ID>:<your read key secret>@[<your vm mycelium IP>]:3900/blobs?region=garage"
```

- Upload your flist to flist bucket using minio (you can use any other client).

```bash
export PATH=$PATH:$HOME/minio-binaries/
mc alias set \
  garage \
  "http://[$MYCELIUM_IP]:3900" \
  "$WRITE_KEY_ID" \
  "$WRITE_KEY_SECRET" \
  --api S3v4

export FLIST_NAME=<"your flist name">

mc cp $FLIST_NAME "s3://flist/$FLIST_NAME"
```

### Serve the flist

- Deploy a name gateway for any domain you want and get the fqdn

```bash
tfcmd deploy gateway name -n "<domain name>" --backends http://[$MYCELIUM_IP]:80
```

- Create Caddyfile

```Caddyfile
http://<fqdn> {
  route /flists/* {
      uri strip_prefix /flists
      reverse_proxy http://127.0.0.1:3902 {
          header_up Host "flist"
      }
  }
  route /blobs/* {
      uri strip_prefix /blobs
      reverse_proxy http://127.0.0.1:3902 {
          header_up Host "blobs"
      }
  }
}
```

- Run `caddy run`
  
Finally, you can get your flist using `https://<fqdn>/flists/<your flist file name>`.
and get your blobs using `https://<fqdn>/blobs/<your blob file name>`.
