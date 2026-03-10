# Neon White Seed Finder

Find seeds for level rushes with given properties.
For example, to find all (up to 200) seeds where "The Clocktower", "Absolution", and "The Third Temple" occur within the first 4 levels,
and "Movement", "Godspeed", and "Waterworks" occur in the last 4, we can do:

```
$ neonwhite_seed_finder 200 ":4>clock,abso,ttt & -4:>move,godsp,water"
INFO: Finding seeds
INFO: Using rush name "White", keeping up to 200 result(s)
INFO: estimated result count: 1; trying to use GPU
DEBUG: selected GPU "Intel(R) Arc(TM) B580 Graphics"
[1873083033,1990387876,]
```

which takes 10 seconds on my GPU.  This is really slow, so hopefully I can figure out why.

If the estimated result count is significantly larger than the max number of results to keep,
it will use a multithreaded CPU version, which will finish as soon as it finds the requested number of seeds
instead of searching all seeds.

The CPU version takes 27 seconds to search all seeds with the above filter on my computer.

## Usage Overview

There are three ways to use the program:
- Find seeds matching a description, if the command line input appears to contain a description
- Find the best seed in terms of reset efficiency (Not yet implemented)
- Simulate a seed

The rush type can be specified with a number (0=White, 1-12=Chapter, 13-15=Sidequest)
or an abbreviated name (White, Mikey, Red, Violet, Yellow, any chapter name, Rainbow, Boss).

The default is White's rush.

You can specify the subcommand (`help`/`find`/`simulate`/`best`) explicitly, or the program will just figure it out.

The max number of results to print is optional and defaults to `1`.
When running on CPU, this lets the program stop as soon as a single valid seed is found.
However, stopping early is not currently supported on GPU.
The program defaults to GPU if the estimated result count is small and a GPU is found.
GPU mode is also not guaranteed to return all results: each GPU thread will ignore any seeds it finds after the first 16 that work.

Run the program with no arguments or `help` for a more detailed description.

## Description Format

The conditions to search for can be specified as a list of rules, separated by `&`, as seen in the example.

Each rule is either a "sequence rule" or "subset rule":
- "sequence rules" are written as `index_range=list,of,level,...,abbreviations`.  These rules require that some range of levels in the shuffle has EXACTLY the specified sequence.
- "subset rules" are written as `index_range>list,of,level,...,abbreviations`.  These rules require that some range of levels in the shuffle contains all the specified levels (in any order).

`index_range` is `a:b`, where `a` is inclusive and `b` is exclusive.  Both are optional and can even be negative, and it basically works like
slice indexing in Python, EXCEPT that for sequence rules, the omitted index is filled in so the length equals the length of the specified sequence.

Level abbreviations can also be specified as indexes (relative to the levels in the rush, not in the whole game).

Valid abbreviations for a lavel name can take any prefix of each word and stick them together (with any/all spaces removed).

The program will yell at you if an abbreviation is ambiguous, or if the rules overlap or are empty/infeasible.

## Installing

Install rust, then run
```
cargo install neonwhite_seed_finder
```
you also need opencl, which is probably part of your GPU drivers but sometimes you will need to install
additional things, for example on Ubuntu you must install `opencl-headers ocl-icd-opencl-dev`,
on Arch Linux `opencl-headers ocl-icd` plus drivers like `intel-compute-runtime`, etc on other distributions.

this will build and install the binary, on Linux it will be in `~/.cargo/bin/`, so that should be in your
path if you want to be able to run `neonwhite_seed_finder` without specifying the full path.

On windows, hopefully cargo sets that up automatically.

Pre-built binaries should be available "soon".

Alternatively, you can build from github instead of crates.io by doing
```
cargo install --git "https://github.com/hacatu/neonwhite_seed_finder" neonwhite_seed_finder
```
or
```
git clone "https://github.com/hacatu/neonwhite_seed_finder"
cd neonwhite_seed_finder
cargo b --profile release
```
and the binary will be in `target/release/`.

## Mathematics

Finding seeds is mostly brute force.  We can simplify the rng a decent amount, but the shuffle will still be hard.

The rng is C#'s default, which is based on Knuth's subtractive rng, which is a lagged fibonacci variant with order 55 mod 2^31-1.

So that means we can represent the nth output as `A[n]*s + B[n] mod 2^31-1` for any seed `s`, for some constants `A[n]` and `B[n]`.

Implementing this naively is about 1% slower than just implementing the rng directly (but within variation).  But we can do a meet in the middle
type thing where we store `A[n]*s_hi` and `A[n]*s_lo + B[n]` and add them together.

## Possible Improvements/TODO:
- Change CPU implementation to include speedups from GPU impl (flatten rules into byte vectors and use meet in the middle to compute rng outputs).  I know this would be about 2x faster since I accidentally ran the opencl code on the CPU before fixing it to select the correct device instead of the first device.
- Add `best` subcommand
- Add GUI
- For a lot of conditions, like "the first x levels must contain these levels", we can greatly simplify shuffling and filtering.  For example, if we only have `k` subset rules, we can just "color" every level one of `k+1` colors and use `ceil(lb(k+1))` bits per element of the shuffle instead of 8.  This is only really worth pursuing in the extreme case when k=1.  That is, we want the first x levels to contain these levels, so we just represent the shuffle as a 96 bit vector with 1s for the levels we want and 0s for others, shuffle this bit vector, and check if the first x bits contain enough 1s at the end.  This is low priority because although it would be easy, it only really works for simple conditions with one subset rule and no sequence rules.  Although basically all realistic queries will be one or two subset rules about the beginning and/or end of the shuffle, so I might do just those cases, since it is likely over 2x faster.

