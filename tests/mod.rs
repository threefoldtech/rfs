use assert_cmd::Command;
use std::{fs, os::unix::prelude::PermissionsExt, path::Path};

const MOUNTPOINT: &str = "/tmp/rmnt";

const FLISTPATH: &str = "/tmp/test.flist";
// const FLISTURL: &str = "https://hub.grid.tf/yasen.3bot/integration_test_fs.flist";
const FLISTURL: &str = "https://hub.grid.tf/azmy.3bot/perm.flist";
// const CHUNKSIZE: usize = 1023;


const PERMMASK:     u32 = 0x1FF;
const EXECONLY:     u32 = 0o111;
const WRITEONLY:    u32 = 0o222;
const WRITEEXEC:    u32 = 0o333;
const READONLY:     u32 = 0o444;
const READEXEC:     u32 = 0o555;
const READWRITE:    u32 = 0o666;
const READWRITEEXEC:u32 = 0o777;

fn download_flist() {
    if Path::new(FLISTPATH).exists() {
        return;
    }
    let client = reqwest::blocking::Client::new();
    let mut response = client.get(FLISTURL).send().unwrap();
    let mut fd = std::fs::File::create(FLISTPATH).unwrap();
    response.copy_to(&mut fd).unwrap();
    
}

fn create_mountpoint_dir() {
    let _ = fs::create_dir_all(MOUNTPOINT);
}

fn remove_mountpoint_dir() {
    let _ = fs::remove_dir_all(MOUNTPOINT);
}

fn cmd_rfs_detached() -> Command {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    cmd.args([
        "-d",
        "--log",
        "/tmp/rfs.logs",
        "--meta",
        FLISTPATH,
        MOUNTPOINT,
    ]);

    cmd
}

fn umount_rfs() {
    Command::new("umount").args([MOUNTPOINT]).unwrap();
}

#[test]
fn test_sucess_mount() {
    download_flist();
    remove_mountpoint_dir();
    create_mountpoint_dir();

    cmd_rfs_detached().assert().success();

    umount_rfs();
}

#[test]
fn test_fs_with_md5sum_check() {
    download_flist();
    remove_mountpoint_dir();
    create_mountpoint_dir();
    cmd_rfs_detached().output().unwrap();
    let current_directory = format!("{}/vfs", MOUNTPOINT);

    Command::new("md5sum")
        .args([
            "-c",
            format!("{}/{}", current_directory, "checksum.md5").as_str(),
        ])
        .current_dir(current_directory)
        .assert()
        .success();

    umount_rfs();
}

#[test]
fn test_symblic_with_md5sum_check() {
    download_flist();
    remove_mountpoint_dir();
    create_mountpoint_dir();
    cmd_rfs_detached().output().unwrap();
    let current_directory = format!("{}/vfs/symbolic_links", MOUNTPOINT);

    Command::new("md5sum")
        .args([
            "-c",
            format!("{}/{}", current_directory, "checksum.md5").as_str(),
        ])
        .current_dir(current_directory)
        .assert()
        .success();

    umount_rfs();
}

#[test]
fn test_permissions() {
    download_flist();
    remove_mountpoint_dir();
    create_mountpoint_dir();
    cmd_rfs_detached().output().unwrap();
    let current_directory = format!("{}/vfs/file_permissions", MOUNTPOINT);

    // all permissions is the same for ugo
    // test read only permission
    let md = fs::metadata(format!("{}/r", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, READONLY);
    // test write only permission
    let md = fs::metadata(format!("{}/w", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, WRITEONLY);
    // test execute only permission
    let md = fs::metadata(format!("{}/x", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, EXECONLY);
    // test read write permission
    let md = fs::metadata(format!("{}/rw", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, READWRITE);
    // test read execute permission
    let md = fs::metadata(format!("{}/rx", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, READEXEC);
    // test write execute permission
    let md = fs::metadata(format!("{}/wx", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, WRITEEXEC);
    // test read write execute permission
    let md = fs::metadata(format!("{}/rwx", current_directory)).unwrap();
    assert_eq!(md.permissions().mode()&PERMMASK, READWRITEEXEC);

    umount_rfs();
}

#[test]
fn test_failure_use_mountpoint_twice() {
    download_flist();
    create_mountpoint_dir();
    cmd_rfs_detached().output().unwrap();
    cmd_rfs_detached().assert().failure();

    umount_rfs();
}

#[test]
fn test_fail_call_bin_without_arguments() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    cmd.assert().failure();
}

#[test]
fn test_fail_call_bin_without_target_argument() {
    download_flist();
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert = cmd.args(["--meta", FLISTPATH]).assert();
    assert.failure();
}

#[test]
fn test_fail_call_bin_without_meta_argument() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert = cmd.arg("/tmp/rmnt").assert();
    assert.failure();
}
