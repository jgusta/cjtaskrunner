#![cfg(unix)]

mod common;

use common::{assert_failure_contains, assert_success, repository_root, temp_path};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct InstallerFixture {
    root: PathBuf,
    releases: PathBuf,
    tools: PathBuf,
    install: PathBuf,
    home: PathBuf,
    curl_log: PathBuf,
}

impl InstallerFixture {
    fn new() -> Self {
        let root = temp_path("installer");
        let releases = root.join("releases");
        let tools = root.join("tools");
        let install = root.join("install");
        let home = root.join("home");
        let curl_log = root.join("curl.log");
        for path in [&releases, &tools, &install, &home] {
            fs::create_dir_all(path).expect("create fixture directory");
        }
        write_executable(
            &tools.join("uname"),
            r#"#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  -s) printf '%s\n' "$MOCK_UNAME_S" ;;
  -m) printf '%s\n' "$MOCK_UNAME_M" ;;
  *) exit 2 ;;
esac
"#,
        );
        write_executable(
            &tools.join("curl"),
            r#"#!/usr/bin/env bash
set -euo pipefail
output=
url=
while (($#)); do
  case "$1" in
    -o|--output)
      output=$2
      shift 2
      ;;
    http://*|https://*)
      url=$1
      shift
      ;;
    *)
      shift
      ;;
  esac
done
test -n "$output"
test -n "$url"
printf '%s\n' "$url" >> "$MOCK_CURL_LOG"
cp "$MOCK_RELEASE_DIR/${url##*/}" "$output"
"#,
        );

        Self {
            root,
            releases,
            tools,
            install,
            home,
            curl_log,
        }
    }

    fn add_archive(&self, platform: &str, archive_kind: &str, executable: &str) {
        let package = format!("cjtaskrunner-{platform}");
        let stage = self.root.join(format!("stage-{platform}"));
        let package_dir = stage.join(&package);
        fs::create_dir_all(&package_dir).expect("create package directory");
        let binary = package_dir.join(executable);
        fs::write(&binary, format!("#!/bin/sh\necho {platform}\n")).expect("write fake cj");
        let mut permissions = fs::metadata(&binary)
            .expect("binary metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary, permissions).expect("make fake cj executable");

        let archive = self.releases.join(format!("{package}.{archive_kind}"));
        let output = if archive_kind == "tar.gz" {
            Command::new("tar")
                .args([
                    "-czf",
                    archive.to_str().expect("archive path"),
                    "-C",
                    stage.to_str().expect("stage path"),
                    &package,
                ])
                .output()
                .expect("create tar archive")
        } else {
            Command::new("zip")
                .args(["-qr", archive.to_str().expect("archive path"), &package])
                .current_dir(&stage)
                .output()
                .expect("create zip archive")
        };
        assert_success(&output);
    }

    fn write_checksums(&self) {
        let mut lines = Vec::new();
        let mut archives = fs::read_dir(&self.releases)
            .expect("read releases")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("SHA256SUMS"))
            .collect::<Vec<_>>();
        archives.sort();
        for archive in archives {
            lines.push(format!(
                "{}  {}",
                sha256(&archive),
                archive.file_name().expect("archive name").to_string_lossy()
            ));
        }
        fs::write(self.releases.join("SHA256SUMS"), lines.join("\n") + "\n")
            .expect("write checksums");
    }

    fn run(&self, os: &str, arch: &str, version: Option<&str>) -> Output {
        let path = format!(
            "{}:{}",
            self.tools.display(),
            std::env::var("PATH").expect("PATH")
        );
        let mut command = Command::new("bash");
        command
            .args(["-c", "cat \"$CJ_INSTALL_SCRIPT\" | bash"])
            .env("CJ_INSTALL_SCRIPT", repository_root().join("install.sh"))
            .env("PATH", path)
            .env("HOME", &self.home)
            .env("CJ_INSTALL_DIR", &self.install)
            .env("MOCK_UNAME_S", os)
            .env("MOCK_UNAME_M", arch)
            .env("MOCK_RELEASE_DIR", &self.releases)
            .env("MOCK_CURL_LOG", &self.curl_log);
        if let Some(version) = version {
            command.env("CJ_VERSION", version);
        }
        command.output().expect("run installer")
    }
}

impl Drop for InstallerFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove installer fixture");
    }
}

#[test]
fn installs_supported_platform_archives_from_release_urls() {
    let cases = [
        (
            "Linux",
            "x86_64",
            "linux-x86_64",
            "tar.gz",
            "cj",
            Some("1.2.3"),
            "releases/download/v1.2.3",
        ),
        (
            "Darwin",
            "x86_64",
            "macos-x86_64",
            "tar.gz",
            "cj",
            None,
            "releases/latest/download",
        ),
        (
            "Darwin",
            "arm64",
            "macos-aarch64",
            "tar.gz",
            "cj",
            Some("v2.0.0-beta.1"),
            "releases/download/v2.0.0-beta.1",
        ),
    ];

    for (os, arch, platform, archive_kind, executable, version, release_path) in cases {
        let fixture = InstallerFixture::new();
        fixture.add_archive(platform, archive_kind, executable);
        fixture.write_checksums();

        let output = fixture.run(os, arch, version);
        assert_success(&output);
        let installed = fixture.install.join(executable);
        assert!(
            installed.exists(),
            "missing installed {}",
            installed.display()
        );
        assert!(
            fs::metadata(&installed)
                .expect("installed metadata")
                .permissions()
                .mode()
                & 0o111
                != 0,
            "{} is not executable",
            installed.display()
        );

        let log = fs::read_to_string(&fixture.curl_log).expect("read curl log");
        let archive = format!("cjtaskrunner-{platform}.{archive_kind}");
        assert!(
            log.contains(&format!(
                "https://github.com/jgusta/cjtaskrunner/{release_path}/{archive}"
            )),
            "missing archive URL in log:\n{log}"
        );
        assert!(
            log.contains(&format!(
                "https://github.com/jgusta/cjtaskrunner/{release_path}/SHA256SUMS"
            )),
            "missing checksum URL in log:\n{log}"
        );
    }
}

#[test]
fn rejects_an_archive_with_the_wrong_checksum() {
    let fixture = InstallerFixture::new();
    fixture.add_archive("linux-x86_64", "tar.gz", "cj");
    fs::write(
        fixture.releases.join("SHA256SUMS"),
        format!("{}  cjtaskrunner-linux-x86_64.tar.gz\n", "0".repeat(64)),
    )
    .expect("write invalid checksum");

    let output = fixture.run("Linux", "x86_64", Some("1.2.3"));
    assert_failure_contains(&output, "checksum verification failed");
    assert!(!fixture.install.join("cj").exists());
}

#[test]
fn rejects_unsupported_platforms_before_downloading() {
    let fixture = InstallerFixture::new();
    let output = fixture.run("Linux", "aarch64", None);

    assert_failure_contains(&output, "unsupported platform");
    assert!(!fixture.curl_log.exists());
}

fn write_executable(path: &Path, source: &str) {
    fs::write(path, source).expect("write executable");
    let mut permissions = fs::metadata(path)
        .expect("executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make executable");
}

fn sha256(path: &Path) -> String {
    for (program, args) in [
        ("sha256sum", vec![path.to_str().expect("checksum path")]),
        (
            "shasum",
            vec!["-a", "256", path.to_str().expect("checksum path")],
        ),
    ] {
        if let Ok(output) = Command::new(program).args(args).output() {
            if output.status.success() {
                return String::from_utf8(output.stdout)
                    .expect("checksum utf8")
                    .split_whitespace()
                    .next()
                    .expect("checksum value")
                    .to_string();
            }
        }
    }
    panic!("sha256sum or shasum is required for installer tests");
}
