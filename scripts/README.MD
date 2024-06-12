# Garage s3 server with flists

## Requirements

- tfcmd
- docker2fl
- rust
- docker
- git
- sqllite
- aws-cli (or any client you want to use)

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
mv ./target/x86_64-unknown-linux-musl/release/rfs /usr/local/bin
```

### Install sqllite

```bash
apt update
apt install sqlite3 
```

### Install aws-cli

```bash
apt-get install unzip
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
./aws/install
```

## Usage

### Deploy garage server

Run garage server using garage server [script](./deploy_garage.sh)

```bash
export DOMAIN=<"your domain name for example threefold">
chmod +x deploy_garage.sh
./deploy_garage.sh
```

This script includes:

1. Deploy a vm with mycelium IP and public IP to run garage s3 server over it.
2. Deploy 2 name gateways for the chosen domain, one to serve the `flist` and the other one for the `blobs`. In the end we should have `flist.$DOMAIN.gateway_domain` and `blobs.$DOMAIN.gateway_domain`
3. Install garage in the vm and add the `domain FQDN from your name gateway -.$DOMAIN.gateway_domain-` to `s3 web root_domain` to be able to serve the server buckets.
4. Run the garage server with the given configuration.

### Manage buckets in garage server

Manage your buckets using manage buckets [script](./manage_buckets.sh)

```bash
export DOMAIN=<"your domain name for example 'threefold'">
chmod +x manage_buckets.sh
./manage_buckets.sh
```

This script includes:

1. Create 2 buckets in garage server one for `flist` and the other for `blobs`.
2. Allow web for both buckets to be able to serve them.
3. Create 2 keys one for write and the othe for read only. The `write-key` will be used to upload the flist and the blobs through rfs. The `read-key` should be updated for flist and blobs to prevent updating them.
4. Adding the keys with their permissions to the bucket.

> *NOTE:*  Don't forget to save your read and write keys (ID and secret).

### Convert docker images to flists

- Convert your image to an flist, The content will be uploaded over blobs buckets

```bash
export IMAGE=<"Your image for example `threefolddev/ubuntu:22.04`">
export WRITE_KEY_ID=<"your key ID">
export WRITE_KEY_SECRET=<"your key secret">
export PUBLIC_IP=<"your machine public IP which has your garage server">

docker2fl -i <Image name> -s 's3://$WRITE_KEY_ID:$WRITE_KEY_SECRET@$PUBLIC_IP:3900/blobs?region=garage'
```

- Update the key to the read only key

```bash
sqlite3 
.open "<your flist file name>"
update route set url="s3://<your read key ID>:<your read key secret>@<your vm public IP>:3900/blobs?region=garage"
```

- Upload your flist to flist bucket using aws (you can use any other client).

```bash
export AWS_ACCESS_KEY_ID=$WRITE_KEY_ID   
export AWS_SECRET_ACCESS_KEY=$WRITE_KEY_SECRET
export AWS_DEFAULT_REGION='garage'
export AWS_ENDPOINT_URL='http://$PUBLIC_IP:3900'

export FLIST_NAME=<"your flist name">

aws s3 cp $FLIST_NAME "s3://flist/$FLIST_NAME"
```

Finally, you can get your flist using `https://flist.$DOMAIN.gateway_domain/<your flist file name>`.
