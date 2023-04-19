# tileserver-rs 🦀

[![The Pipeline](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/pipeline.yml/badge.svg)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/pipeline.yml)

<img src="./.github/assets/tileserver-rs.png" width="512" height="512" align="center" alt="tileserver-rs logo" />

## Features

- Supports serving vector tiles using `.mbtiles` | `.pmtiles` as source
- Built using [Axum](https://github.com/tokio-rs/axum) & [Nuxt 3](https://nuxt.com/)
- Vector maps with GL styles
- Map Tile Server for consumption in OpenLayers, mapbox-gl-js, maplibre-gl-js and more.

## Table of Contents

- [tileserver-rs 🦀](#tileserver-rs-)
  - [Features](#features)
  - [Table of Contents](#table-of-contents)
  - [Requirements](#requirements)
  - [Usage](#usage)
    - [Docker](#docker)
  - [Contributing](#contributing)
  - [Author](#author)
    - [Spepipelineal thanks to](#spepipelineal-thanks-to)

## Requirements

- [Rust 1.66+](https://www.rust-lang.org/)
- (Optional) [Docker](https://www.docker.com/)

## Usage

### Docker

```sh
docker run --rm -it -v /your/local/config/path:/data -p 8080:8080 vinayakkulkarni/tileserver-rs
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

### Spepipelineal thanks to

- [tileserver-gl](https://github.com/maptiler/tileserver-gl)
