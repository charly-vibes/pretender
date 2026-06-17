#!/usr/bin/env sh
for file in "$@"; do
  awk -v file="$file" 'length > 100 {
    printf "{\"file\":\"%s\",\"line\":%d,\"message\":\"line exceeds 100 chars (%d)\",\"code\":\"LL001\"}\n",
      file, NR, length
  }' "$file"
done
