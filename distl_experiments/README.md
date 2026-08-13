# RV 2026 Artifacts

This is the code for the RV 2026 paper "Runtime Monitoring of Distributed Cyber-Physical Systems Without a Global Clock".

Experiments for this paper were run on a machine with a AMD Ryzen 7 PRO 7840U CPU and 16 GB of LPDDR5 RAM. Expected execution times listed in this README are based on this machine.

## Directory Setup

We require the following directory layout:

some-parent-directory/ \
|- distl/ \
|- distl_experiments/

`some-parent-directory/` can be named whatever you prefer, but installation requires `distl` and `distl_experiments` to be named as such.

All following commands in this README assume the current working directory is `distl_experiments`.

## Installation with Docker

With Docker installed, building a Docker image of the software is a single command. From the top directory of the software package:

```
docker build --tag distl -f Dockerfile ..
```

This will take approximately 30 seconds to build.

## Running all benchmarks with Docker

If on Linux, to run the benchmarks, run the following:

```
docker run --rm -v $(pwd)/results:/usr/src/app/experiments/results -it distl ./gen_results.sh
```

For running on Windows PowerShell, the command must be slightly modified:

```
docker run --rm -v "${PWD}/results:/usr/src/app/experiments/results" -it distl ./gen_results.sh
```

This will take approximately 2.5 minutes to run, and will provide results in a `results` directory. `f*-results.csv` is data for formulas `f1`, `f2`, and `f3` when varying `N` (Figure 4 of the paper), and `f*-eps-results.csv` is data for the formulas when varying epsilon (Figure 5 of the paper).

- To analyze any of the results, the reader can choose to plot the data with any plotting tool of their choice (such as Excel or matplotlib, for example).

These benchmarks are a subset of the results generated for the paper.
