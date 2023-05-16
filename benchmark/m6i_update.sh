#!/bin/sh

sudo apt update
sudo apt upgrade -y

sudo apt install python3.9 python3-pip -y
python3.9 -m pip install numpy scipy msgspec pymatching

cd /home/ubuntu/fusion-blossom
git pull
cargo build --release
