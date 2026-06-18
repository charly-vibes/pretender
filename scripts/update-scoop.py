#!/usr/bin/env python3
"""Create or update a Scoop manifest for pretender."""
import json
import os
import sys

manifest_path = sys.argv[1]
version = sys.argv[2]
tag = sys.argv[3]
checksums_path = sys.argv[4]

sha_win = None
with open(checksums_path) as f:
    for line in f:
        parts = line.split()
        if len(parts) == 2 and "windows_amd64" in parts[1]:
            sha_win = parts[0]
            break

if sha_win is None:
    print("ERROR: windows_amd64 checksum not found", file=sys.stderr)
    sys.exit(1)

url = f"https://github.com/charly-vibes/pretender/releases/download/{tag}/pretender_{version}_windows_amd64.zip"

manifest = {
    "version": version,
    "description": "Structural code-quality checker for multiple languages",
    "homepage": "https://github.com/charly-vibes/pretender",
    "license": "MIT",
    "url": url,
    "hash": f"sha256:{sha_win}",
    "bin": "pretender.exe",
    "checkver": {
        "github": "https://github.com/charly-vibes/pretender"
    },
    "autoupdate": {
        "url": "https://github.com/charly-vibes/pretender/releases/download/v$version/pretender_$version_windows_amd64.zip"
    }
}

os.makedirs(os.path.dirname(manifest_path), exist_ok=True)
with open(manifest_path, "w") as f:
    json.dump(manifest, f, indent=4)
    f.write("\n")

print(f"Wrote {manifest_path} (version {version})")
print(f"  url: {url}")
print(f"  sha256: {sha_win[:16]}...")
