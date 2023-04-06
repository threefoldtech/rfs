use anyhow::{Context, Result};
use assert_cmd::Command;
use std::{
    fs,
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
};

const TMP_PATH: &str = "/tmp";

const MOUNTPOINT: &str = "/tmp/rmnt";

const FLISTURL: &str = "https://hub.grid.tf/azmy.3bot/perm.flist";

const PERMMASK: u32 = 0x1FF;
const EXECONLY: u32 = 0o111;
const WRITEONLY: u32 = 0o222;
const WRITEEXEC: u32 = 0o333;
const READONLY: u32 = 0o444;
const READEXEC: u32 = 0o555;
const READWRITE: u32 = 0o666;
const READWRITEEXEC: u32 = 0o777;

struct TestMount<'a>(PathBuf, &'a str);

impl<'a> TestMount<'a> {
    pub fn new(flist: &'a str, mountpoint: &'a str) -> Result<Self> {
        fs::create_dir_all(mountpoint).context("failed to create test mountpoint")?;

        let (_, name) = flist
            .rsplit_once("/")
            .ok_or(anyhow::anyhow!("invalid flist path"))?;
        let path = Path::new(TMP_PATH).join(name);
        if !path.exists() {
            let client = reqwest::blocking::Client::new();
            let mut response = client.get(flist).send().context("failed to get flist")?;
            let mut fd = std::fs::File::create(&path).context("failed to create temp flist")?;
            response
                .copy_to(&mut fd)
                .context("failed to download flist")?;
        }

        Ok(TestMount(path, mountpoint))
    }

    pub fn mount(&self) -> Command {
        let mut cmd = Command::cargo_bin("rfs").unwrap();
        cmd.arg("-d")
            .arg("--log")
            .arg("/tmp/test-rmb.log")
            .arg("--meta")
            .arg(&self.0)
            .arg(self.1);

        cmd
    }

    fn unmount(&self) {
        let _ = Command::new("umount").arg(self.1).output();
    }
}

impl<'a> Drop for TestMount<'a> {
    fn drop(&mut self) {
        self.unmount();
    }
}

#[test]
fn test_sucess_mount() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();

    mops.mount().assert().success();
}

#[test]
fn test_fs_with_md5sum_check() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();
    mops.mount().output().unwrap();

    Command::new("md5sum")
        .args(["-c", "checksum.md5"])
        .current_dir(MOUNTPOINT)
        .assert()
        .success();
}

#[test]
fn test_symbolic_with_md5sum_check() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();
    mops.mount().output().unwrap();
    let current_directory = format!("{}/symbolic_links", MOUNTPOINT);

    Command::new("md5sum")
        .args(["-c", "checksum.md5"])
        .current_dir(current_directory)
        .assert()
        .success();
}

#[test]
#[ignore]
fn test_permissions() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();

    mops.mount().output().unwrap();
    let current_directory = format!("{}/file_permissions", MOUNTPOINT);

    // all permissions is the same for ugo
    // test read only permission
    let md = fs::metadata(format!("{}/r", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, READONLY);
    // test write only permission
    let md = fs::metadata(format!("{}/w", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, WRITEONLY);
    // test execute only permission
    let md = fs::metadata(format!("{}/x", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, EXECONLY);
    // test read write permission
    let md = fs::metadata(format!("{}/rw", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, READWRITE);
    // test read execute permission
    let md = fs::metadata(format!("{}/rx", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, READEXEC);
    // test write execute permission
    let md = fs::metadata(format!("{}/wx", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, WRITEEXEC);
    // test read write execute permission
    let md = fs::metadata(format!("{}/rwx", current_directory)).unwrap();
    assert_eq!(md.permissions().mode() & PERMMASK, READWRITEEXEC);
}

#[test]
fn test_failure_use_mountpoint_twice() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();

    mops.mount().output().unwrap();
    mops.mount().assert().failure();
}

#[test]
fn test_fail_call_bin_without_arguments() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    cmd.assert().failure();
}

#[test]
fn test_fail_call_bin_without_target_argument() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert = cmd.arg("--meta").arg(&mops.0).assert();
    assert.failure();
}

#[test]
fn test_fail_call_bin_without_meta_argument() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert = cmd.arg("/tmp/rmnt").assert();
    assert.failure();
}

#[tokio::test]
async fn test_walk() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();
    use rfs::meta::{EntryKind, Metadata, Walk, WalkVisitor};
    let meta = Metadata::open(&mops.0).await.unwrap();

    //let mut paths = vec![];

    struct TestVisitor {
        paths: Vec<PathBuf>,
    }
    #[async_trait::async_trait]
    impl WalkVisitor for TestVisitor {
        async fn visit<P: AsRef<Path> + Send + Sync>(
            &mut self,
            path: P,
            entry: &rfs::meta::Entry,
        ) -> Result<Walk> {
            self.paths.push(path.as_ref().to_owned());

            if path.as_ref() == Path::new("/file_5M.random") {
                assert!(matches!(entry.kind, EntryKind::File(_)));
            } else if path.as_ref() == Path::new("/file_permissions") {
                assert!(matches!(entry.kind, EntryKind::Dir(_)));
            } else if path.as_ref() == Path::new("/symbolic_links/file_5M.random") {
                assert!(matches!(entry.kind, EntryKind::Link(_)));
            }

            Ok(Walk::Continue)
        }
    }

    let mut visitor = TestVisitor { paths: vec![] };
    //let inner = Arc::clone(&paths);
    meta.walk(&mut visitor).await.unwrap();

    assert_eq!(24, visitor.paths.len());
}

#[tokio::test]
async fn test_extract() {
    let mops = TestMount::new(FLISTURL, MOUNTPOINT).unwrap();
    use rfs::cache::Cache;
    use rfs::meta::Metadata;

    let meta = Metadata::open(&mops.0).await.unwrap();

    let cache = Cache::new("redis://hub.grid.tf:9900", "/tmp/ex-cache")
        .await
        .unwrap();

    let path = Path::new("/tmp/extracted");
    let _ = fs::remove_dir_all(path);
    rfs::extract(&meta, &cache, path).await.unwrap();

    Command::new("md5sum")
        .args(["-c", "checksum.md5"])
        .current_dir(path)
        .assert()
        .success();

    Command::new("md5sum")
        .args(["-c", "checksum.md5"])
        .current_dir(path.join("symbolic_links"))
        .assert()
        .success();
}
