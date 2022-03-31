use assert_cmd::{assert::Assert, Command};
use std::fs;

const MOUNTPOINT: &str = "/tmp/rmnt";

fn create_mountpoint_dir() {
    fs::create_dir_all(MOUNTPOINT).unwrap();
}

fn remove_mountpoint_dir() {
    fs::remove_dir_all(MOUNTPOINT).unwrap();
}

fn run_rfs_detached() -> Assert {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert_mount = cmd
        .args([
            "-d",
            "--log",
            "/tmp/rfs.logs",
            "--meta",
            "./tests/threefoldtech-funk-latest.flist",
            MOUNTPOINT,
        ])
        .assert();

    assert_mount
}

fn umount_rfs() {
    nix::mount::umount(MOUNTPOINT).unwrap();
}

#[test]
fn test_sucess_mount() {
    remove_mountpoint_dir();
    create_mountpoint_dir();
    
    run_rfs_detached().success();

    umount_rfs();
}

#[test]
fn test_failure_use_mountpoint_twice() {
    create_mountpoint_dir();
    run_rfs_detached();
    run_rfs_detached().failure();
    
    umount_rfs();
}

#[test]
fn test_fail_call_bin_without_arguments() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    cmd.assert().failure();
}

#[test]
fn test_fail_call_bin_without_target_argument() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert = cmd
        .args(["--meta", "./threefoldtech-funk-latest.flist"])
        .assert();
    assert.failure();
}

#[test]
fn test_fail_call_bin_without_meta_argument() {
    let mut cmd = Command::cargo_bin("rfs").unwrap();
    let assert = cmd.arg("/tmp/rmnt").assert();
    assert.failure();
}
