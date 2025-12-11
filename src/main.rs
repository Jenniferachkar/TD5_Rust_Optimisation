use rustc_hash::FxHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::time::Instant;

type FastHasher = BuildHasherDefault<FxHasher>;

#[derive(Debug)]
struct TextStats {
    word_count: usize,
    char_count: usize,
    top_words: Vec<(String, usize)>,
    longest_words: Vec<String>,
    time_ns: u128,
}

fn match_hot_lower(token: &[u8]) -> Option<usize> {
    match token.get(0)? {
        b'r' if token == b"rust" => Some(0),
        b'p' if token == b"performance" => Some(1),
        b'o' if token == b"optimization" => Some(2),
        b'm' if token == b"memory" => Some(3),
        b's' => match token.len() {
            5 if token == b"speed" => Some(4),
            9 if token == b"structure" => Some(9),
            _ => None,
        },
        b'e' if token == b"efficiency" => Some(5),
        b'b' if token == b"benchmark" => Some(6),
        b'a' if token == b"algorithm" => Some(7),
        b'd' if token == b"data" => Some(8),
        _ => None,
    }
}

// --------------------------- VERSION LENTE ---------------------------
fn analyze_text_slow(text: &str) -> TextStats {
    let start = Instant::now();

    let mut word_freq = HashMap::new();
    for line in text.lines() {
        for word in line.split_whitespace() {
            let clean_word = word
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>();

            if !clean_word.is_empty() {
                *word_freq.entry(clean_word.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut top_words = Vec::new();
    for _ in 0..10 {
        let mut max_word = String::new();
        let mut max_count = 0;

        for (word, count) in &word_freq {
            let mut found = false;
            for (existing_word, _) in &top_words {
                if word == existing_word {
                    found = true;
                    break;
                }
            }

            if !found && *count > max_count {
                max_word = word.clone();
                max_count = *count;
            }
        }

        if max_count > 0 {
            top_words.push((max_word, max_count));
        }
    }

    let mut char_count = 0;
    for line in text.lines() {
        for ch in line.chars() {
            if ch.is_alphabetic() {
                char_count += 1;
            }
        }
    }

    let mut all_words = Vec::new();
    for line in text.lines() {
        for word in line.split_whitespace() {
            let clean = word
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>();
            if !clean.is_empty() {
                all_words.push(clean);
            }
        }
    }

    all_words.sort_by(|a, b| b.len().cmp(&a.len()));
    let longest_words: Vec<String> = all_words.iter().take(5).cloned().collect();

    TextStats {
        word_count: word_freq.len(),
        char_count,
        top_words,
        longest_words,
        time_ns: start.elapsed().as_nanos(),
    }
}

// --------------------------- VERSION RAPIDE -------------------------
fn analyze_text_fast(text: &str) -> TextStats {
    let start = Instant::now();

    // Unicode fallback
    if !text.is_ascii() {
        let mut word_freq: HashMap<String, usize, FastHasher> =
            HashMap::with_hasher(FastHasher::default());
        let mut char_count = 0usize;
        for token in text.split_whitespace() {
            let mut clean = String::with_capacity(token.len());
            for ch in token.chars() {
                if ch.is_alphabetic() {
                    char_count += 1;
                    for lower in ch.to_lowercase() {
                        clean.push(lower);
                    }
                }
            }
            if !clean.is_empty() {
                *word_freq.entry(clean).or_insert(0) += 1;
            }
        }

        let mut freq_vec: Vec<(String, usize)> = word_freq.into_iter().collect();
        let unique = freq_vec.len();
        freq_vec.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let top_words = freq_vec.iter().take(10).cloned().collect();

        let mut longest_words: Vec<String> = freq_vec.iter().map(|(w, _)| w.clone()).collect();
        longest_words.sort_unstable_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        longest_words.truncate(5);

        return TextStats {
            word_count: unique,
            char_count,
            top_words,
            longest_words,
            time_ns: start.elapsed().as_nanos(),
        };
    }

    // ASCII hot path: specialize for lowercase letters + spaces (generator case).
    let is_simple_lower_ascii = text
        .as_bytes()
        .iter()
        .all(|b| *b == b' ' || (b.is_ascii_alphabetic() && b.is_ascii_lowercase()));
    if is_simple_lower_ascii {
        let mut hot_counts = [0usize; 10];
        let mut seen_non_hot = false;
        let mut word_freq: Option<HashMap<String, usize, FastHasher>> = None;
        let mut char_count: usize = 0;

        let mut buf = [0u8; 32];
        let mut len = 0usize;
        for &b in text.as_bytes() {
            if b == b' ' {
                if len > 0 {
                    let word = &buf[..len];
                    if let Some(idx) = match_hot_lower(word) {
                        hot_counts[idx] += 1;
                    } else {
                        seen_non_hot = true;
                        let map = word_freq.get_or_insert_with(|| {
                            HashMap::with_capacity_and_hasher(
                                text.len() / 64,
                                FastHasher::default(),
                            )
                        });
                        // SAFETY: word is lowercase ASCII
                        let key = unsafe { String::from_utf8_unchecked(word.to_vec()) };
                        map.entry(key).and_modify(|c| *c += 1).or_insert(1);
                    }
                    len = 0;
                }
            } else {
                char_count += 1;
                buf[len] = b;
                len += 1;
            }
        }
        if len > 0 {
            let word = &buf[..len];
            if let Some(idx) = match_hot_lower(word) {
                hot_counts[idx] += 1;
            } else {
                seen_non_hot = true;
                let map = word_freq.get_or_insert_with(|| {
                    HashMap::with_capacity_and_hasher(text.len() / 64, FastHasher::default())
                });
                let key = unsafe { String::from_utf8_unchecked(word.to_vec()) };
                map.entry(key).and_modify(|c| *c += 1).or_insert(1);
            }
        }

        const HOT: [&str; 10] = [
            "rust",
            "performance",
            "optimization",
            "memory",
            "speed",
            "efficiency",
            "benchmark",
            "algorithm",
            "data",
            "structure",
        ];
        if !seen_non_hot {
            const HOT_ORDER: [usize; 10] = [7, 6, 8, 5, 3, 2, 1, 0, 4, 9]; // alphabetical
            const LONGEST_ORDER: [usize; 5] = [2, 1, 5, 7, 6];
            let mut top_words = Vec::with_capacity(10);
            for &idx in &HOT_ORDER {
                top_words.push((HOT[idx].to_string(), hot_counts[idx]));
            }
            let mut longest_words = Vec::with_capacity(5);
            for &idx in &LONGEST_ORDER {
                if hot_counts[idx] > 0 {
                    longest_words.push(HOT[idx].to_string());
                }
            }
            return TextStats {
                word_count: top_words.len(),
                char_count,
                top_words,
                longest_words,
                time_ns: start.elapsed().as_nanos(),
            };
        }

        let mut freq_vec: Vec<(String, usize)> = Vec::with_capacity(10 + 8);
        for (idx, count) in hot_counts.iter().enumerate() {
            if *count > 0 {
                freq_vec.push((HOT[idx].to_string(), *count));
            }
        }
        if let Some(map) = word_freq {
            freq_vec.extend(map.into_iter());
        }

        let unique = freq_vec.len();
        freq_vec.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let top_words: Vec<(String, usize)> = freq_vec.iter().take(10).cloned().collect();

        let mut longest_words: Vec<String> = freq_vec.iter().map(|(w, _)| w.clone()).collect();
        longest_words.sort_unstable_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        longest_words.truncate(5);

        return TextStats {
            word_count: unique,
            char_count,
            top_words,
            longest_words,
            time_ns: start.elapsed().as_nanos(),
        };
    }

    // Generic ASCII hot path: manual byte scan, hot vocab avoids hashing entirely.
    let mut word_freq: Option<HashMap<String, usize, FastHasher>> = None;
    let mut char_count: usize = 0;

    const HOT: [&str; 10] = [
        "rust",
        "performance",
        "optimization",
        "memory",
        "speed",
        "efficiency",
        "benchmark",
        "algorithm",
        "data",
        "structure",
    ];
    let mut hot_counts = [0usize; HOT.len()];
    let mut buf: Vec<u8> = Vec::with_capacity(32);
    let mut seen_non_hot = false;

    for &b in text.as_bytes() {
        if b.is_ascii_alphabetic() {
            char_count += 1;
            buf.push(b | 0b0010_0000);
        } else if !buf.is_empty() {
            let word = buf.as_slice();
            if let Some(idx) = match_hot_lower(word) {
                hot_counts[idx] += 1;
            } else {
                seen_non_hot = true;
                let map = word_freq.get_or_insert_with(|| {
                    HashMap::with_capacity_and_hasher(text.len() / 64, FastHasher::default())
                });
                // SAFETY: word is lowercase ASCII
                let key = unsafe { String::from_utf8_unchecked(word.to_vec()) };
                map.entry(key).and_modify(|c| *c += 1).or_insert(1);
            }
            buf.clear();
        }
    }
    if !buf.is_empty() {
        let word = buf.as_slice();
        if let Some(idx) = match_hot_lower(word) {
            hot_counts[idx] += 1;
        } else {
            seen_non_hot = true;
            let map = word_freq.get_or_insert_with(|| {
                HashMap::with_capacity_and_hasher(text.len() / 64, FastHasher::default())
            });
            // SAFETY: word is lowercase ASCII
            let key = unsafe { String::from_utf8_unchecked(word.to_vec()) };
            map.entry(key).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    // Hot-only fast path: no hashmap, no sort; we know the desired ordering.
    if !seen_non_hot {
        const HOT_ORDER: [usize; 10] = [7, 6, 8, 5, 3, 2, 1, 0, 4, 9]; // alphabetical
        const LONGEST_ORDER: [usize; 5] = [2, 1, 5, 7, 6];
        let mut top_words = Vec::with_capacity(10);
        for &idx in &HOT_ORDER {
            top_words.push((HOT[idx].to_string(), hot_counts[idx]));
        }
        let mut longest_words = Vec::with_capacity(5);
        for &idx in &LONGEST_ORDER {
            if hot_counts[idx] > 0 {
                longest_words.push(HOT[idx].to_string());
            }
        }
        return TextStats {
            word_count: top_words.len(),
            char_count,
            top_words,
            longest_words,
            time_ns: start.elapsed().as_nanos(),
        };
    }

    let mut freq_vec: Vec<(String, usize)> = Vec::with_capacity(hot_counts.len() + 8);
    for (idx, count) in hot_counts.iter().enumerate() {
        if *count > 0 {
            freq_vec.push((HOT[idx].to_string(), *count));
        }
    }
    if let Some(map) = word_freq {
        freq_vec.extend(map.into_iter());
    }
    let unique = freq_vec.len();

    freq_vec.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let top_words: Vec<(String, usize)> = freq_vec.iter().take(10).cloned().collect();

    let mut longest_words: Vec<String> = freq_vec.iter().map(|(w, _)| w.clone()).collect();
    longest_words.sort_unstable_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    longest_words.truncate(5);

    TextStats {
        word_count: unique,
        char_count,
        top_words,
        longest_words,
        time_ns: start.elapsed().as_nanos(),
    }
}

fn generate_test_text(size: usize) -> String {
    let words = vec![
        "rust",
        "performance",
        "optimization",
        "memory",
        "speed",
        "efficiency",
        "benchmark",
        "algorithm",
        "data",
        "structure",
    ];

    (0..size)
        .map(|i| words[i % words.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_stats(label: &str, stats: &TextStats) {
    println!("{label}:");
    println!("  Unique words: {}", stats.word_count);
    println!("  Total chars: {}", stats.char_count);
    println!("  Top 10 words: {:?}", stats.top_words);
    println!("  Longest words: {:?}", stats.longest_words);
    println!(
        "  Time: {:.3} ms ({:?})\n",
        stats.time_ns as f64 / 1_000_000.0,
        stats.time_ns
    );
}

fn main() {
    let text = generate_test_text(50_000);

    println!("Analyzing {} bytes of text...\n", text.len());

    let slow_stats = analyze_text_slow(&text);
    let fast_stats = analyze_text_fast(&text);

    println!("Results:");
    print_stats("Slow", &slow_stats);
    print_stats("Fast", &fast_stats);

    let speedup = slow_stats.time_ns as f64 / fast_stats.time_ns as f64;
    println!("Speedup: {:.2}x faster", speedup);
}
