use bollard::auth::DockerCredentials;
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, RemoveContainerOptions,
};
use bollard::image::{CreateImageOptions, RemoveImageOptions};
use bollard::Docker;
use std::sync::mpsc::Sender;
use tempdir::TempDir;
use walkdir::WalkDir;

use anyhow::{Context, Result};
use futures_util::stream::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use std::default::Default;
use std::fs;
use std::path::Path;
use std::process::Command;
use tokio_async_drop::tokio_async_drop;

use crate::fungi::Writer;
use crate::store::Store;

struct DockerInfo {
    image_name: String,
    container_name: String,
    docker: Docker,
}

impl Drop for DockerInfo {
    fn drop(&mut self) {
        tokio_async_drop!({
            let res = clean(&self.docker, &self.image_name, &self.container_name)
                .await
                .context("failed to clean docker image and container");

            if res.is_err() {
                log::error!(
                    "cleaning docker image and container failed with error: {:?}",
                    res.err()
                );
            }
        });
    }
}

pub struct DockerImageToFlist {
    meta: Writer,
    image_name: String,
    credentials: Option<DockerCredentials>,
    docker_tmp_dir: TempDir,
}

impl DockerImageToFlist {
    pub fn new(
        meta: Writer,
        image_name: String,
        credentials: Option<DockerCredentials>,
        docker_tmp_dir: TempDir,
    ) -> Self {
        Self {
            meta,
            image_name,
            credentials,
            docker_tmp_dir,
        }
    }

    pub fn files_count(&self) -> u32 {
        WalkDir::new(self.docker_tmp_dir.path())
            .into_iter()
            .count() as u32
    }

    pub async fn prepare(&mut self) -> Result<()> {
        #[cfg(unix)]
        let docker = Docker::connect_with_socket_defaults().context("failed to create docker")?;

        let container_file =
            Path::file_stem(self.docker_tmp_dir.path()).expect("failed to get directory name");
        let container_name = container_file
            .to_str()
            .expect("failed to get container name")
            .to_owned();

        let docker_info = DockerInfo {
            image_name: self.image_name.to_owned(),
            container_name,
            docker,
        };

        extract_image(
            &docker_info.docker,
            &docker_info.image_name,
            &docker_info.container_name,
            self.docker_tmp_dir.path(),
            self.credentials.clone(),
        )
        .await
        .context("failed to extract docker image to a directory")?;
        log::info!(
            "docker image '{}' is extracted successfully",
            docker_info.image_name
        );

        Ok(())
    }

    pub async fn pack<S: Store>(&mut self, store: S, sender: Option<Sender<u32>>) -> Result<()> {
        crate::pack(
            self.meta.clone(),
            store,
            &self.docker_tmp_dir.path(),
            true,
            sender,
        )
        .await
        .context("failed to pack flist")?;

        log::info!("flist has been created successfully");
        Ok(())
    }

    pub async fn convert<S: Store>(&mut self, store: S, sender: Option<Sender<u32>>) -> Result<()> {
        self.prepare().await?;
        self.pack(store, sender).await?;

        Ok(())
    }
}

async fn extract_image(
    docker: &Docker,
    image_name: &str,
    container_name: &str,
    docker_tmp_dir_path: &Path,
    credentials: Option<DockerCredentials>,
) -> Result<()> {
    pull_image(docker, image_name, credentials).await?;
    create_container(docker, image_name, container_name)
        .await
        .context("failed to create docker container")?;
    export_container(container_name, docker_tmp_dir_path)
        .context("failed to export docker container")?;
    container_boot(docker, container_name, docker_tmp_dir_path)
        .await
        .context("failed to boot docker container")?;

    Ok(())
}

async fn pull_image(
    docker: &Docker,
    image_name: &str,
    credentials: Option<DockerCredentials>,
) -> Result<()> {
    log::info!("pulling docker image {}", image_name);

    let options = Some(CreateImageOptions {
        from_image: image_name,
        ..Default::default()
    });

    let mut image_pull_stream = docker.create_image(options, None, credentials);
    while let Some(msg) = image_pull_stream.next().await {
        msg.context("failed to pull docker image")?;
    }

    Ok(())
}

async fn create_container(docker: &Docker, image_name: &str, container_name: &str) -> Result<()> {
    log::debug!("Inspecting docker image configurations {}", image_name);

    let image = docker
        .inspect_image(image_name)
        .await
        .context("failed to inspect docker image")?;
    let image_config = image.config.context("failed to get docker image configs")?;

    let mut command = "";
    if image_config.cmd.is_none() && image_config.entrypoint.is_none() {
        command = "/bin/sh";
    }

    log::debug!("Creating a docker container {}", container_name);

    let options = Some(CreateContainerOptions {
        name: container_name,
        platform: None,
    });

    let config = Config {
        image: Some(image_name),
        hostname: Some(container_name),
        cmd: Some(vec![command]),
        ..Default::default()
    };

    docker
        .create_container(options, config)
        .await
        .context("failed to create docker temporary container")?;

    Ok(())
}

fn export_container(container_name: &str, docker_tmp_dir_path: &Path) -> Result<()> {
    log::debug!("Exporting docker container {}", container_name);

    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "docker export {} | tar -xpf - -C {}",
            container_name,
            docker_tmp_dir_path.display()
        ))
        .output()
        .expect("failed to execute export docker container");

    Ok(())
}

async fn container_boot(
    docker: &Docker,
    container_name: &str,
    docker_tmp_dir_path: &Path,
) -> Result<()> {
    log::debug!(
        "Inspecting docker container configurations {}",
        container_name
    );

    let options = Some(InspectContainerOptions { size: false });
    let container = docker
        .inspect_container(container_name, options)
        .await
        .context("failed to inspect docker container")?;

    let container_config = container
        .config
        .context("failed to get docker container configs")?;

    let mut command = String::new();
    let mut args: Vec<String> = Vec::new();
    let mut env: HashMap<String, String> = HashMap::new();
    let mut cwd = String::from("/");

    if let Some(ref entrypoint) = container_config.entrypoint {
        if !entrypoint.is_empty() {
            command = entrypoint[0].to_string();
            for i in 1..entrypoint.len() {
                args.push(entrypoint[i].to_string());
            }
        }
    }

    if let Some(ref cmd) = container_config.cmd {
        if !cmd.is_empty() {
            if command.is_empty() {
                command = cmd[0].to_string();
                for i in 1..cmd.len() {
                    args.push(cmd[i].to_string());
                }
            } else {
                for i in 0..cmd.len() {
                    args.push(cmd[i].to_string());
                }
            }
        }
    }

    if command.is_empty() {
        command = String::from("/bin/sh");
    }

    if let Some(envs) = container_config.env {
        for entry in envs.iter() {
            if let Some((key, value)) = entry.split_once('=') {
                env.insert(key.to_string(), value.to_string());
            }
        }
    }

    if let Some(ref working_dir) = container_config.working_dir {
        if !working_dir.is_empty() {
            cwd = working_dir.to_string();
        }
    }

    let metadata = json!({
        "startup": {
            "entry": {
                "name": "core.system",
                "args": {
                    "name": command,
                    "args": args,
                    "env": env,
                    "dir": cwd,
                }
            }
        }
    });

    let toml_metadata: toml::Value = serde_json::from_str(&metadata.to_string())?;

    log::info!(
        "Creating '.startup.toml' file from container {} contains {}",
        container_name,
        toml_metadata.to_string()
    );

    fs::write(
        docker_tmp_dir_path.join(".startup.toml"),
        toml_metadata.to_string(),
    )?;

    Ok(())
}

async fn clean(docker: &Docker, image_name: &str, container_name: &str) -> Result<()> {
    log::debug!("Removing docker container {}", container_name);

    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });

    docker
        .remove_container(container_name, options)
        .await
        .context("failed to remove docker container")?;

    log::debug!("Removing docker image {}", image_name);

    let options = Some(RemoveImageOptions {
        force: true,
        ..Default::default()
    });

    docker
        .remove_image(image_name, options, None)
        .await
        .context("failed to remove docker image")?;

    Ok(())
}
