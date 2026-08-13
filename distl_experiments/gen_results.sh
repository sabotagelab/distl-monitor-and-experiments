#!/usr/bin/env fish

python3 datagen.py 3 30 10 data
./reset.sh
./run.sh
./reset.sh
./run_eps.sh
./reset.sh
