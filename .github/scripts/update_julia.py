#!/usr/bin/env python3

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Update Julia's MMTk dependency metadata and checksums."
    )
    parser.add_argument("julia_path", help="Path to the local Julia checkout")
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


def run_command(args: list[str], cwd: Path) -> None:
    subprocess.run(args, cwd=cwd, check=True)


def update_version_file(version_file: Path, version: str, commit: str) -> None:
    content = version_file.read_text(encoding="utf-8")

    updated, sha_count = re.subn(
        r'(^MMTK_JULIA_SHA1\s*=\s*)[0-9a-f]+$',
        rf"\g<1>{commit}",
        content,
        count=1,
        flags=re.MULTILINE,
    )
    if sha_count != 1:
        raise RuntimeError(f"Could not update MMTK_JULIA_SHA1 in {version_file}")

    updated, tar_count = re.subn(
        r'(^MMTK_JULIA_TAR_URL\s*=\s*https://github\.com/mmtk/mmtk-julia/archive/refs/tags/v)[^"]+(\.tar\.gz)$',
        rf"\g<1>{version}\2",
        updated,
        count=1,
        flags=re.MULTILINE,
    )
    if tar_count != 1:
        raise RuntimeError(f"Could not update MMTK_JULIA_TAR_URL in {version_file}")

    updated, jll_count = re.subn(
        r'(^MMTK_JULIA_JLL_VER\s*:?=\s*)[0-9][^\s]*$',
        rf"\g<1>{version}+0",
        updated,
        count=1,
        flags=re.MULTILINE,
    )
    if jll_count != 1:
        raise RuntimeError(f"Could not update MMTK_JULIA_JLL_VER in {version_file}")

    version_file.write_text(updated, encoding="utf-8")


def read_checksum_pair(checksum_dir: Path) -> tuple[str, str]:
    md5_file = checksum_dir / "md5"
    sha512_file = checksum_dir / "sha512"
    if not md5_file.is_file() or not sha512_file.is_file():
        raise RuntimeError(f"Missing checksum files in {checksum_dir}")
    return (
        md5_file.read_text(encoding="utf-8").strip(),
        sha512_file.read_text(encoding="utf-8").strip(),
    )


def write_packed_checksums(
    checksums_file: Path,
    version: str,
    commit: str,
    bb_md5: str,
    bb_sha512: str,
    src_md5: str,
    src_sha512: str,
) -> None:
    bb_basename = f"mmtk_julia.v{version}+0.x86_64-linux-gnu.tar.gz"
    src_basename = f"mmtk_julia-{commit}.tar.gz"
    lines = [
        f"{bb_basename}/md5/{bb_md5}",
        f"{bb_basename}/sha512/{bb_sha512}",
        f"{src_basename}/md5/{src_md5}",
        f"{src_basename}/sha512/{src_sha512}",
    ]
    checksums_file.write_text("\n".join(lines) + "\n", encoding="utf-8")


def cleanup_generated_checksums(*paths: Path) -> None:
    for path in paths:
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            path.unlink()


def refresh_checksums(julia_path: Path) -> None:
    run_command(
        [
            "make",
            "-C",
            "deps",
            "USE_BINARYBUILDER_MMTK_JULIA=0",
            "FC_VERSION=7.0.0",
            "DEPS_GIT=0",
            "checksum-mmtk_julia",
        ],
        cwd=julia_path,
    )
    run_command(
        [
            "make",
            "-C",
            "deps",
            "USE_BINARYBUILDER_MMTK_JULIA=1",
            "MMTK_JULIA_BB_TRIPLET=x86_64-linux-gnu",
            "DEPS_GIT=0",
            "checksum-mmtk_julia",
        ],
        cwd=julia_path,
    )


def main() -> int:
    args = parse_args()

    repo_root = script_root()
    cargo_toml = repo_root / "mmtk" / "Cargo.toml"
    julia_path = Path(args.julia_path).resolve()
    version_file = julia_path / "deps" / "mmtk_julia.version"
    checksums_root = julia_path / "deps" / "checksums"
    checksums_file = checksums_root / "mmtk_julia"

    if not cargo_toml.is_file():
        raise RuntimeError(f"Cargo.toml not found: {cargo_toml}")
    if not (julia_path / ".git").exists():
        raise RuntimeError(f"Not a git repository: {julia_path}")
    if not version_file.is_file():
        raise RuntimeError(f"Version file not found: {version_file}")
    if not checksums_root.is_dir():
        raise RuntimeError(f"Checksums directory not found: {checksums_root}")

    version = read_version(cargo_toml)
    commit = git_output(repo_root, "rev-parse", "HEAD")

    update_version_file(version_file, version, commit)
    refresh_checksums(julia_path)

    bb_dir = checksums_root / f"mmtk_julia.v{version}+0.x86_64-linux-gnu.tar.gz"
    src_dir = checksums_root / f"mmtk_julia-{commit}.tar.gz"
    if not bb_dir.is_dir():
        raise RuntimeError(
            f"Expected BinaryBuilder checksum directory was not generated: {bb_dir}"
        )
    if not src_dir.is_dir():
        raise RuntimeError(f"Expected source checksum directory was not generated: {src_dir}")

    bb_md5, bb_sha512 = read_checksum_pair(bb_dir)
    src_md5, src_sha512 = read_checksum_pair(src_dir)
    write_packed_checksums(
        checksums_file,
        version,
        commit,
        bb_md5,
        bb_sha512,
        src_md5,
        src_sha512,
    )
    cleanup_generated_checksums(bb_dir, src_dir)

    print(f"Updated {version_file}")
    print(f"Updated {checksums_file}")
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
