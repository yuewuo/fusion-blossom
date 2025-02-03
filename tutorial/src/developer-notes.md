
# How to include image in README.md

The most elegant way I found is to follow this post: [https://stackoverflow.com/a/42677655](https://stackoverflow.com/a/42677655).
At specific version, the image referred to is a fixed link.



# To save Github build resources

The most expensive build action is for macOS:
Your 2,420.00 included minutes used consists of 342.00 Ubuntu 2-core minutes, 328.00 Windows 2-core minutes, and 1,750.00 macOS 3-core minutes.

Since I'm using Mac for development already, I can run the following command to build it locally.

```sh
rustup target add x86_64-apple-darwin  # only once

CIBW_BUILD=cp38-* MACOSX_DEPLOYMENT_TARGET=10.12 CIBW_ARCHS_MACOS=universal2 python -m cibuildwheel --output-dir target
```
