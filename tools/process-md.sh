#!/bin/bash

echo "Processing markdown files.."

for file in md/*.md; do pandoc -t html5 --metadata title="$(basename "$file" .md)" --standalone --self-contained "$file" -o "content/$(basename "$file" .md).html"; echo "$(basename "$file" .md)"; done
