# The Slow Text Analyzer – Fast/Slow Comparison

This repo contains a deliberately slow text analyzer and an optimized fast path. Run both and compare:

```bash
cargo run --release
```

By default it analyzes a generated 50k-word text, prints stats for the slow and fast implementations, and reports the speedup.

## Why the original function was slow

1) Multiple full passes over the text
The slow version scanned the input three times: once to count word frequencies, again to count alphabetic chars, and again to rebuild all words for “longest”. Each pass thrashes caches (see “Make your programs run faster by better using the data cache”).  
**Fast version** processes everything in a single pass, improving spatial/temporal locality and cutting cache misses.

2) Excessive allocations and cloning
Slow code did `to_lowercase()` + `chars().filter().collect()` + `clone()` for every token, creating millions of tiny allocations and iterator overhead (see “Make your programs run faster – avoid function calls”).  
**Fast version** builds cleaned words once with a reusable buffer and inserts without cloning, eliminating most allocations and call overhead.

3) O(n²) top-10 algorithm  
Slow code repeatedly scanned the whole HashMap and the selected list to find the next max, giving quadratic behavior as data grows.  
**Fast version** materializes the map once into a vector and uses an O(n log n) sort to get the top 10.

4) Rebuilding all words to find the longest  
Slow code re-created a full vector of all cleaned words even though they already existed as map keys, doubling memory traffic and hurting cache behavior (see the data-cache article).  
**Fast version**  reuses existing keys to pick the longest, avoiding duplicates and extra scans.

5) Unicode-heavy branching in tight loops  
Using `is_alphabetic()` / `to_lowercase()` introduces complex branching that’s hard to predict (see “How branches influence performance…”).  
**Fast version** sticks to ASCII-only checks (`is_ascii_alphabetic`, bit-lowercasing) for predictable, branch-friendly loops.

## Summary of optimizations in the fast version

- One-pass processing → better data locality, fewer cache misses.  
- Reduced allocations → faster inner loop, fewer function calls.  
- Simple ASCII checks → predictable branches.  
- Efficient top-10 via sort → O(n log n) instead of O(n²).  
- Reuse existing data → less memory traffic, better cache performance.  
- Inspired by:  
  - https://johnnysswlab.com/make-your-programs-run-faster-by-better-using-the-data-cache/  
  - https://johnnysswlab.com/make-your-programs-run-faster-avoid-function-calls/  
  - https://johnnysswlab.com/how-branches-influence-the-performance-of-your-code-and-what-can-you-do-about-it/

## How to run

```bash
cargo run --release
```

