#!/usr/bin/env python3
"""Create or update a Homebrew formula for pretender."""
import sys
import os

formula_path = sys.argv[1]
version = sys.argv[2]
tag = sys.argv[3]
checksums_path = sys.argv[4]

shas = {}
with open(checksums_path) as f:
    for line in f:
        parts = line.split()
        if len(parts) == 2:
            sha, name = parts
            for platform in ["darwin_arm64", "darwin_amd64", "linux_arm64", "linux_amd64"]:
                if platform in name:
                    shas[platform] = sha

base = f"https://github.com/charly-vibes/pretender/releases/download/{tag}"

formula = f"""\
# typed: false
# frozen_string_literal: true

class Pretender < Formula
  desc "Structural code-quality checker for multiple languages"
  homepage "https://github.com/charly-vibes/pretender"
  version "{version}"
  license "MIT"

  on_macos do
    on_arm do
      url "{base}/pretender_{version}_darwin_arm64.tar.gz"
      sha256 "{shas['darwin_arm64']}"
    end
    on_intel do
      url "{base}/pretender_{version}_darwin_amd64.tar.gz"
      sha256 "{shas['darwin_amd64']}"
    end
  end

  on_linux do
    on_arm do
      if Hardware::CPU.is_64_bit?
        url "{base}/pretender_{version}_linux_arm64.tar.gz"
        sha256 "{shas['linux_arm64']}"
      end
    end
    on_intel do
      url "{base}/pretender_{version}_linux_amd64.tar.gz"
      sha256 "{shas['linux_amd64']}"
    end
  end

  def install
    bin.install "pretender"
  end

  test do
    system "\#{{bin}}/pretender", "--version"
  end
end
"""

os.makedirs(os.path.dirname(formula_path), exist_ok=True)
with open(formula_path, "w") as f:
    f.write(formula)

print(f"Wrote {formula_path} (version {version})")
for p, s in shas.items():
    print(f"  {p}: {s[:16]}...")
