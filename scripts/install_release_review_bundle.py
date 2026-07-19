#!/usr/bin/env python3
"""Verify and install an independently authored release-review archive."""

from __future__ import annotations

import argparse
import hashlib
import os
import pathlib
import re
import shutil
import tarfile
import tempfile


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def _validated_members(archive: tarfile.TarFile) -> list[tarfile.TarInfo]:
    members = archive.getmembers()
    seen: set[str] = set()
    for member in members:
        if "\\" in member.name:
            raise ValueError(f"unsafe archive path {member.name!r}")
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or not path.parts or ".." in path.parts:
            raise ValueError(f"unsafe archive path {member.name!r}")
        if path.parts[0] != "reviews":
            raise ValueError(f"archive payload is outside reviews/: {member.name!r}")
        normalized = path.as_posix()
        if normalized in seen:
            raise ValueError(f"duplicate archive path {normalized!r}")
        seen.add(normalized)
        if member.issym() or member.islnk():
            raise ValueError(f"archive links are forbidden: {member.name!r}")
        if not member.isfile() and not member.isdir():
            raise ValueError(f"unsupported archive member type: {member.name!r}")
    required = {
        "reviews/findings.json",
        "reviews/maintainer-signoff.toml",
    }
    missing = required.difference(seen)
    if missing:
        raise ValueError(f"review archive is missing required paths: {sorted(missing)}")
    return members


def install_review_bundle(
    archive_path: pathlib.Path | str,
    expected_sha256: str,
    output_path: pathlib.Path | str,
) -> None:
    archive_path = pathlib.Path(archive_path)
    output_path = pathlib.Path(output_path)
    if not SHA256_RE.fullmatch(expected_sha256):
        raise ValueError("expected review archive SHA-256 must be 64 lowercase hex characters")
    actual_sha256 = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    if actual_sha256 != expected_sha256:
        raise ValueError(
            f"review archive SHA-256 does not match: expected {expected_sha256}, got {actual_sha256}"
        )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = pathlib.Path(
        tempfile.mkdtemp(prefix=f".{output_path.name}-", dir=output_path.parent)
    )
    try:
        with tarfile.open(archive_path, "r:gz") as archive:
            members = _validated_members(archive)
            for member in members:
                relative = pathlib.PurePosixPath(member.name)
                destination = temporary.joinpath(*relative.parts)
                if member.isdir():
                    destination.mkdir(parents=True, exist_ok=True)
                    continue
                destination.parent.mkdir(parents=True, exist_ok=True)
                source = archive.extractfile(member)
                if source is None:
                    raise ValueError(f"could not read archive member {member.name!r}")
                with source, destination.open("wb") as target:
                    shutil.copyfileobj(source, target)
        if output_path.exists():
            shutil.rmtree(output_path)
        os.replace(temporary, output_path)
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", required=True, type=pathlib.Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    install_review_bundle(args.archive, args.sha256, args.output)


if __name__ == "__main__":
    main()
