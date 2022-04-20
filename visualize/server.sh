#!/bin/sh 

# Absolute path to this script, e.g. /home/user/bin/foo.sh
SCRIPT=$(readlink -f "$0")
# Absolute path this script is in, thus /home/user/bin
SCRIPTPATH=$(dirname "$SCRIPT")

echo "running server to host folder $SCRIPTPATH"
cd $SCRIPTPATH

python3 -m http.server 8066
