# Installation

There are multiple ways to install this library.
Choose any one of the methods below that best suit your needs.

## Python Package

This is the easiest way to use the library.
All the demos are in Python, but once you become familiar with the Python interface, the Rust native interface is exactly the same.

```shell
pip3 install fusion-blossom
```

## Build from Source using Rust

This is the recommended way for experts to install the library, for two reasons.
First, the pre-compiled binary doesn't include the Blossom V library due to incompatible license, and thus is not capable of running the verifier that invokes the blossom V library to double check the correctness of fusion blossom library.
Second, you can access all the internal details of the library and reproduce the results in our paper.

### Download the Blossom V Library [Optional]

Please note that you're responsible for requesting a proper license for the use of this library, as well as obeying any requirement.
In order to use the Blossom V algorithm as a verifier, you need to download the @@kolmogorov2009blossom library from [https://pub.ist.ac.at/~vnk/software.html](https://pub.ist.ac.at/~vnk/software.html) into the `blossomV` folder in the repository.

An example of downloading the Blossom V library is below. Note that the latest version number and website address may change over time.

```bash
wget -c https://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz -O - | tar -xz
cp -r blossom5-v2.05.src/* blossomV/
rm -r blossom5-v2.05.src
```

You don't need to compile the Blossom V library manually.

Note that you can still compile the project without the Blossom V library.
The build script automatically detects whether the Blossom V library exists and enables the feature accordingly.

### Install the Rust Toolchain

We need the Rust toolchain to compile the project written in the Rust programming language.
Please see [https://rustup.rs/](https://rustup.rs/) for the latest instructions.
An example on Unix-like operating systems is below.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.bashrc  # this will add `~/.cargo/bin` to path
```

After installing the Rust toolchain successfully, you can compile the library and binary by

```bash
cargo build --release
cargo run --release -- test serial  # randomized test using blossom V as a verifier
```

### Install the Python Development Tools [Optional]

If you want to develop the Python module, you need a few more tools

```bash
sudo apt install python3 python3-pip
pip3 install maturin
maturin develop  # build the Python package and install in your virtualenv or conda
python3 scripts/demo.py  # run a demo using the installed library
```

### Install Frontend tools [Optional]

The frontend is a single-page application using Vue.js and Three.js frameworks.
To use the frontend, you need to start a local server.
Then visit [http://localhost:8066/?filename=primal_module_serial_basic_10.json](http://localhost:8066/?filename=primal_module_serial_basic_10.json) you should see a visualization of the solving process.
If you saw errors like `fetch file error`, then it means you haven't generated that visualization file.
Simply run `cargo test primal_module_serial_basic_10` to generate the required file or run `cargo test` to generate all visualization files for the test cases.

```sh
python3 visualize/server.py
```

### Install mdbook to build this tutorial [Optional]

In order to build this tutorial, you need to install [mdbook](https://crates.io/crates/mdbook) and several plugins.

```bash
cargo install mdbook
cargo install mdbook-bib
cd tutorial
mdbook serve  # dev mode, automatically refresh the local web page on code change
mdbook build  # build deployment in /docs folder, to be recognized by GitHub Pages
```
