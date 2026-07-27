#!/usr/bin/env python3

import argparse
import re
import subprocess
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Update the Yggdrasil mmtk_julia recipe to the current mmtk-julia version and commit."
    )
    parser.add_argument("yggdrasil_path", help="Path to the local Yggdrasil checkout")
    return parser.parse_args()


def script_root() -> Path:
    return Path(__file__).resolve().parents[2]


def read_version(cargo_toml: Path) -> str:
    content = cargo_toml.read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
    if match is None:
        raise RuntimeError(f"Could not find version in {cargo_toml}")
    return match.group(1)


def git_output(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def ensure_branch(repo: Path, branch_name: str) -> None:
    existing = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "--verify", branch_name],
        capture_output=True,
        text=True,
    )
    if existing.returncode == 0:
        subprocess.run(
            ["git", "-C", str(repo), "switch", branch_name],
            check=True,
        )
        return

    subprocess.run(
        ["git", "-C", str(repo), "switch", "-c", branch_name],
        check=True,
    )


def update_build_tarballs(build_tarballs: Path, version: str, commit: str) -> None:
    content = build_tarballs.read_text(encoding="utf-8")

    version_updated, version_count = re.subn(
        r'(^version\s*=\s*v")[^"]+(")',
        rf"\g<1>{version}\2",
        content,
        count=1,
        flags=re.MULTILINE,
    )
    if version_count != 1:
        raise RuntimeError(f"Could not update version in {build_tarballs}")

    hash_updated, hash_count = re.subn(
        r'(GitSource\("https://github\.com/mmtk/mmtk-julia\.git",\s*")[0-9a-f]+("\))',
        rf"\g<1>{commit}\2",
        version_updated,
        count=1,
    )
    if hash_count != 1:
        raise RuntimeError(f"Could not update GitSource hash in {build_tarballs}")

    build_tarballs.write_text(hash_updated, encoding="utf-8")


def main() -> int:
    args = parse_args()

    repo_root = script_root()
    cargo_toml = repo_root / "mmtk" / "Cargo.toml"
    yggdrasil_path = Path(args.yggdrasil_path).resolve()
    build_tarballs = yggdrasil_path / "M" / "mmtk_julia" / "build_tarballs.jl"

    if not cargo_toml.is_file():
        raise RuntimeError(f"Cargo.toml not found: {cargo_toml}")
    if not (yggdrasil_path / ".git").exists():
        raise RuntimeError(f"Not a git repository: {yggdrasil_path}")
    if not build_tarballs.is_file():
        raise RuntimeError(f"build_tarballs.jl not found: {build_tarballs}")

    version = read_version(cargo_toml)
    branch_name = f"mmtk-julia/v{version.replace('.', '')}"
    commit = git_output(repo_root, "rev-parse", "HEAD")

    ensure_branch(yggdrasil_path, branch_name)
    update_build_tarballs(build_tarballs, version, commit)

    print(f"Updated {build_tarballs}")
    print(f"Branch: {branch_name}")
    print(f"Version: {version}")
    print(f"Commit: {commit}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        print(error, file=sys.stderr)
        raise SystemExit(error.returncode)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
