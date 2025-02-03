# Change Log

See [this](https://keepachangelog.com/en/1.0.0/) for format of writing change logs.

## 0.1.3 (2022.10.11)

The first version of Python package

- [x] continuous integration for dev and main branch at https://jenkins.fusionblossom.com/
- [x] build Python wheels (py37, abi3) for every commit in main branch using GitHub Action

## 0.1.4 (2022.10.17)

- [x] change `example.rs` to `example_codes.rs` for clarity
- [x] start writing a tutorial using `mdbook`
- [ ] publish `fusion_blossom` package to crate.io
- [ ] add `dangerous_pointer` feature that improve speed by ~20%

## 0.2.9 (2024.4.23)

- add `max_tree_size` option to implement a spectrum of decoders between UF and MWPM

## 0.2.10 (2024.5.7)

- optimize Python interface to accept `max_tree_size = None`

## 0.2.13 (2025.2.1)

- use bottle=0.14-dev to fix error on python3.13 where cgi module is deprecated
