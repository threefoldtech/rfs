fn main() {
    println!(
        "cargo:rustc-env=GIT_VERSION={}",
        git_version::git_version!(args = ["--tags", "--always", "--dirty=-modified"])
    );
}
