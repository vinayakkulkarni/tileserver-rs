<img src="./.github/assets/tileserver-rs.png" width="512" height="512" align="center" alt="tileserver-rs logo" />

# TileServer

[![CI](https://img.shields.io/github/actions/workflow/status/vinayakkulkarni/tileserver-rs/ci.yml?logo=github-actions&branch=main)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/ci.yml)
[![CodeQL](https://img.shields.io/github/actions/workflow/status/vinayakkulkarni/tileserver-rs/codeql.yml?logo=github-actions&branch=main)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/codeql.yml)
[![DeepScan grade](https://deepscan.io/api/teams/9055/projects/18331/branches/446995/badge/grade.svg)](https://deepscan.io/dashboard#view=project&tid=9055&pid=18331&bid=446995)
[![Snyk Vulnerabilities for GitHub Repo](https://img.shields.io/snyk/vulnerabilities/github/vinayakkulkarni/tileserver-rs)](https://snyk.io/test/github/vinayakkulkarni/tileserver-rs)
[![GitHub contributors](https://img.shields.io/github/contributors/vinayakkulkarni/tileserver-rs?logo=github)](https://github.com/vinayakkulkarni/tileserver-rs/graphs/contributors)

[![Crates.io](https://img.shields.io/crates/v/tileserver-rs?logo=rust)](https://crates.io/tileserver-rs)

## Features

- Supports serving vector tiles using `.mbtiles` | `.pmtiles` as source
- Built using [Actix](https://actix.rs/) & [Rust](https://www.rust-lang.org/)
- Vector maps with GL styles
- Map Tile Server for consumption in OpenLayers, mapbox-gl-js, maplibre-gl-js and more.
-

## Table of Contents

- [TileServer](#tileserver)
  - [Features](#features)
  - [Table of Contents](#table-of-contents)
  - [Requirements](#requirements)
  - [Usage](#usage)
    - [Docker](#docker)
  - [Contributing](#contributing)
  - [Author](#author)
    - [Special thanks to](#special-thanks-to)

## Requirements

- [Rust 1.66+](https://www.rust-lang.org/)
- (Optional) [Docker](https://www.docker.com/)

## Usage

### Docker

```sh
docker run --rm -it -v /your/local/config/path:/data -p 8080:8080 vinayakkulkarin/tileserver-rs
```

## Contributing

1. Fork it ( [https://github.com/vinayakkulkarni/tileserver-rs/fork](https://github.com/vinayakkulkarni/tileserver-rs/fork) )
2. Create your feature branch (`git checkout -b feat/new-feature`)
3. Commit your changes (`git commit -Sam 'feat: add feature'`)
4. Push to the branch (`git push origin feat/new-feature`)
5. Create a new [Pull Request](https://github.com/vinayakkulkarni/tileserver-rs/compare)

_Note_:

1. Please contribute using [GitHub Flow](https://web.archive.org/web/20191104103724/https://guides.github.com/introduction/flow/)
2. Commits & PRs will be allowed only if the commit messages & PR titles follow the [conventional commit standard](https://www.conventionalcommits.org/), _read more about it [here](https://github.com/conventional-changelog/commitlint/tree/master/%40commitlint/config-conventional#type-enum)_
3. PS. Ensure your commits are signed. _[Read why](https://withblue.ink/2020/05/17/how-and-why-to-sign-git-commits.html)_

## Author

**tileserver-rs** &copy; [Vinayak](https://vinayakkulkarni.dev), Released under the [MIT](./LICENSE) License.<br>
Authored and maintained by Vinayak Kulkarni with help from contributors ([list](https://github.com/vinayakkulkarni/tileserver-rs/contributors)).

> [vinayakkulkarni.dev](https://vinayakkulkarni.dev) · GitHub [@vinayakkulkarni](https://github.com/vinayakkulkarni) · Twitter [@\_vinayak_k](https://twitter.com/_vinayak_k)

### Special thanks to

- [tileserver-gl](https://github.com/maptiler/tileserver-gl)
