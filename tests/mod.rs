use assert_cmd::{assert::Assert, Command};

const MOUNTPOINT: &str = "/tmp/rmnt";

fn create_mountpoint_dir() {
    Command::new("/bin/mkdir").args(["-p", MOUNTPOINT]).output().unwrap();
}

fn remove_mountpoint_dir() {
    Command::new("/bin/rm").args(["-Rf", MOUNTPOINT]).output().unwrap();
}

fn run_rfs_detached() -> Assert {
    Command::new("sudo").output().unwrap();
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

    let logs = Command::new("cat").args(["/tmp/rfs.logs"]).output();
    println!("\r\n\r\n==========\r\nDetaching the filesystem's logs\r\n{:#?}", logs);
    assert_mount
}

fn umount_rfs() {
    Command::new("umount").args([MOUNTPOINT]).unwrap();
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
