"""Inspect reviewed source metadata without extracting files or caching tar headers."""

import json
from pathlib import Path
import re
import struct
import sys
import tarfile
import zipfile

MAX_JSON_BYTES = 1024 * 1024
MAX_NAME_LENGTH = 32768
MAX_ENTRIES = 5000000
MAX_ZIP_METADATA_BYTES = 32 * 1024 * 1024
MAX_ZIP_ENTRIES = 200000


class BoundedTarInfo(tarfile.TarInfo):
    @classmethod
    def fromtarfile(cls, archive):
        try:
            return super().fromtarfile(archive)
        except (tarfile.InvalidHeaderError, tarfile.EmptyHeaderError,
                tarfile.TruncatedHeaderError) as error:
            # TarFile.next otherwise treats damaged later headers as end-of-archive.
            raise tarfile.ReadError(str(error)) from error

    def _proc_member(self, archive):
        metadata_types = (tarfile.XHDTYPE, tarfile.XGLTYPE,
                          tarfile.GNUTYPE_LONGNAME, tarfile.GNUTYPE_LONGLINK)
        if self.type in metadata_types and self.size > MAX_JSON_BYTES:
            raise ValueError("Tar extended metadata exceeds 1 MiB")
        return super()._proc_member(archive)


def check_zip_metadata(path):
    # Check the central-directory allocation before ZipFile reads its metadata.
    with path.open("rb") as source:
        source.seek(0, 2)
        size = source.tell()
        source.seek(max(0, size - 65557))
        tail = source.read(65557)
    offset = tail.rfind(b"PK\x05\x06")
    if offset < 0 or len(tail) - offset < 22:
        raise ValueError("ZIP end-of-directory record is missing")
    _, disk, directory_disk, disk_entries, entries, length, start, comment = struct.unpack_from(
        "<4s4H2LH", tail, offset)
    if disk or directory_disk or disk_entries != entries:
        raise ValueError("Split ZIP archives are unsupported")
    if entries == 65535 or length == 0xffffffff or start == 0xffffffff:
        raise ValueError("ZIP64 source metadata is unsupported")
    if entries > MAX_ZIP_ENTRIES or length > MAX_ZIP_METADATA_BYTES:
        raise ValueError("ZIP central-directory metadata exceeds the supported limit")
    if start + length > size or offset + 22 + comment != len(tail):
        raise ValueError("ZIP end-of-directory metadata is inconsistent")


def inspect(path, required):
    matched = set()
    count = 0

    def visit(name):
        nonlocal count
        if not isinstance(name, str) or not name or len(name) > MAX_NAME_LENGTH:
            raise ValueError("Source archive contains an empty or oversized member name")
        count += 1
        if count > MAX_ENTRIES:
            raise ValueError("Source archive entry count exceeds the supported limit")
        while name.startswith("./"):
            name = name[2:]
        name = name.removesuffix("/")
        if name in required:
            matched.add(name)

    if path.name.lower().endswith(".zip"):
        check_zip_metadata(path)
        with zipfile.ZipFile(path) as archive:
            for member in archive.infolist():
                visit(member.filename)
    else:
        with tarfile.open(path, mode="r|*", encoding="utf-8", errors="strict",
                          tarinfo=BoundedTarInfo) as archive:
            while (member := archive.next()) is not None:
                visit(member.name)
                # Python 3.12 next() advances by offset; no cached lookup is used.
                archive.members.clear()
    missing = [name for name in required if name not in matched]
    if missing:
        raise ValueError("Source archive is missing required members: " + ", ".join(missing[:8]))
    return {"archiveEntryCount": count,
            "matchedMembers": [name for name in required if name in matched]}


def main():
    if sys.version_info < (3, 12):
        raise ValueError("Source archive inspection requires Python 3.12 or newer")
    if len(sys.argv) != 2:
        raise ValueError("Usage: python -I inspect-source-archive.py <archive>")
    payload = sys.stdin.buffer.read(MAX_JSON_BYTES + 1)
    if len(payload) > MAX_JSON_BYTES:
        raise ValueError("Required-member metadata exceeds 1 MiB")
    required = json.loads(payload.decode("utf-8"))
    if not isinstance(required, list) or not required or any(
            not isinstance(name, str) or not name or len(name) > MAX_NAME_LENGTH
            or re.search(r'[\\:*?"<>|\x00-\x1f\x7f]', name)
            or any(segment in ("", ".", "..") or segment.endswith((" ", "."))
                   for segment in name.split("/")) for name in required):
        raise ValueError("Required members must be nonempty safe relative paths")
    if len(set(required)) != len(required):
        raise ValueError("Required members must be unique")
    result = json.dumps(inspect(Path(sys.argv[1]), required), ensure_ascii=True)
    if len(result) > MAX_JSON_BYTES:
        raise ValueError("Source inspection result exceeds 1 MiB")
    sys.stdout.buffer.write((result + "\n").encode("utf-8"))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        archive_name = Path(sys.argv[1]).name if len(sys.argv) > 1 else "<unspecified>"
        message = f"{archive_name}: {type(error).__name__}: {error}"
        sys.stderr.buffer.write((message[:8192] + "\n").encode("utf-8", errors="backslashreplace"))
        sys.exit(1)
