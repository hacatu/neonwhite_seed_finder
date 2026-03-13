# Neon White Seed Finder

[![Latest version](https://img.shields.io/crates/v/neonwhite_seed_finder.svg)](https://crates.io/crates/neonwhite_seed_finder)

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

The program now has a GUI!  If you double click it (or run with no arguments), it will open the gui

<img src="https://raw.githubhsercontent.com/hacatu/neonwhite_seed_finder/refs/heads/master/resources/ttt_ct_abso_water_move_godsp.png"> &nbsp; &nbsp; <img src="https://raw.githubhsercontent.com/hacatu/neonwhite_seed_finder/refs/heads/master/resources/jdroach_test.png">

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

Run the program with `help` for a more detailed description.

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

If you want to disable the cli or the gui, compile with `--no-default-features --features gui` or `--no-default-features --features cli` (if you disable both, it will be challenging to use the program).

## Mathematics

Finding seeds is mostly brute force.  We can simplify the rng a decent amount, but the shuffle will still be hard.

The rng is C#'s default, which is based on Knuth's subtractive rng, which is a lagged fibonacci variant with order 55 mod 2^31-1.

So that means we can represent the nth output as `A[n]*s + B[n] mod 2^31-1` for any seed `s`, for some constants `A[n]` and `B[n]`.

Implementing this naively is about 1% slower than just implementing the rng directly (but within variation).

I have tried a lot of optimizations, but so far nothing has been meaningfully faster:
- Split `s = s_hi + s_lo` and store a table of `A[n]*s_lo + B[n]` for every `s_lo, n` so that each thread computes `A[n]*s_hi` only once and then for each rng output we just have to do one memory load and one 32 bit modulo add.  Computing `A[n]*s_lo + B[n] mod 2^31-1` is not actually slower because we can actually do it with 32 bit multiplication and some bitshifts.
- Flatten the rules on the cpu like we do on the gpu so that it's faster to read them.
- Combine some of the rules to create the most powerful single subset rule that is implied by our rules, so that we can pre-filter with only bitvecs.  Since most of the cost is computing all the rng outputs, this turns out to not help (though I may have messed it up).

## Possible Improvements/TODO:
- Add `best` subcommand (currently, neonlite/eventtracker do not seem to output to latest.log or any other file during a rush, so I would probably have to fork eventtracker or look into livesplit (though livesplit is not suitable because it does not work with shuffle mode))
- Add a force stop button.  Killing the worker thread will almost certainly not clean up the openCL environment if it is set up, and even
making it a separate process and killing that might leave dangling resources, so this would probably require making the worker thread do work in
small chunks so it can check for a kill flag, and it seems not worth it.

