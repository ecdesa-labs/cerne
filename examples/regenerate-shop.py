#!/usr/bin/env python3
"""Generates examples/shop again from docs/en/tutorial.md.

Runs the tutorial's `cerne` commands with the CLI of this checkout, writes the
`<!-- file: .. -->` blocks over the generated files, points the `cerne` dependency
at crates/cerne and runs cargo fmt.

    python3 examples/regenerate-shop.py
"""

import os
import re
import shutil
import subprocess

workspace = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
examples = os.path.join(workspace, "examples")
shop = os.path.join(examples, "shop")
cli = os.path.join(workspace, "target", "debug", "cerne")

# --- The CLI of this checkout -------------------------------------------------

subprocess.run(["cargo", "build", "-q", "-p", "cerne-cli"], cwd=workspace, check=True)

shutil.rmtree(shop, ignore_errors=True)

# --- The tutorial: the cerne commands and the filled files --------------------

with open(os.path.join(workspace, "docs", "en", "tutorial.md")) as tutorial:
    lines = tutorial.read().split("\n")

project = examples
previous = ""
index = 0

while index < len(lines):
    line = lines[index]

    if line.startswith("```"):
        language = line[3:]
        code = []
        index += 1

        while lines[index] != "```":
            code.append(lines[index])
            index += 1

        if language == "bash" and not previous.startswith("<!--"):
            for command in code:
                if command.startswith("cd "):
                    project = os.path.join(project, command[3:])
                elif command.startswith("cerne "):
                    subprocess.run([cli] + command[6:].split(), cwd=project, check=True)

        filled = re.fullmatch(r"<!-- file: (.*) -->", previous)

        if filled:
            with open(os.path.join(project, filled.group(1)), "w") as file:
                file.write("\n".join(code) + "\n")

    previous = line
    index += 1

# --- Cargo.toml: crates/cerne instead of crates.io ----------------------------

manifest = os.path.join(shop, "Cargo.toml")

with open(manifest) as file:
    cargo_toml = file.read()

cargo_toml = re.sub(r'cerne = \{ version = "[^"]*"', 'cerne = { path = "../../crates/cerne"', cargo_toml)
cargo_toml = cargo_toml.replace('edition = "2024"\n', 'edition = "2024"\npublish = false\n', 1)

with open(manifest, "w") as file:
    file.write(cargo_toml)

# --- cargo fmt: the CLI adds each `pub mod` in the order it creates them -------

subprocess.run(["cargo", "fmt", "-p", "shop"], cwd=workspace, check=True)
