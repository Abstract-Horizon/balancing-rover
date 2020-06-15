#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

DIR=$DIR/src/python/balance

echo ""
echo Uploading     balance
pyros $1 upload -s balance $DIR/balance_main.py -e $DIR/balancing.py $DIR/accel.py $DIR/gyro.py
echo Restarting    balance
pyros $1 restart   balance

echo ""
echo "Currently running processes:"
pyros $1 ps