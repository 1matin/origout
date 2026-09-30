## Origout

You can read the whitepaper of Origout [here](WHITEPAPER.md).

## CLI

Run `origout identity` to show the current public key, or `origout identity --raw` to print only the key. Run `origout --help` for command help.

The Checkmate module declares the command tree with a `defineCli` megaprogram, which expands to calls to the `origout.cli` capability in `schemas/origout.cm`. Its `origout.app.Run` entry receives the process arguments and returns output and an exit code to the Rust host.

## Copyright

Origout: A Decentralized, Authoritative Git Collaboration Protocol Copyright (C) 2026 Matin Mohammadi

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied
warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
details. You should have received a copy of the GNU Affero General Public License along with this program. If not,
see <https://www.gnu.org/licenses/>.
