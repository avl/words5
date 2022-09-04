extern crate anyhow;
extern crate indexmap;
use rayon::prelude::*;
use anyhow::Result;
use indexmap::IndexSet;
use std::collections::{HashMap};
use std::fmt::{Debug, Formatter};
use std::io::Write;
use std::mem::take;
use std::ops::{BitOrAssign, Range};
use std::sync::{Mutex};
use std::sync::atomic::{AtomicU64, Ordering};


/// The basic idea of this program is to do an exhaustive search very efficiently.
///
/// One simple optimization is to first get rid of all anagrams (we later recreate the
/// solutions lost because of this).
///
/// We first loop through all words. For each word, we determine which other words
/// do not share a letter with this word. We then continue with the third, fourth
/// and fifth word. At each stage, we eliminate all words which may no longer be part
/// of the solution (because they contain already-used letters). We also cache this set
/// of still-usable words. Since there could be multiple choices for the first and second word (for example),
/// which in total use the same letters, this caching saves on computations which would otherwise
/// have been repeated.
///
/// As a further optimization, we keep track of choices of first word that yield no solutions.
/// All states reached while considering this are marked in a bitmap as 'dead ends'.
/// For example, neither the word 'clank' nor 'dirge' (or any anagrams of them) are part of any solution.
/// This means that if we ever end up in a situation where we have used the letters 'acdegiklnr', we know
/// there can be no solutions. Even if we didn't use the words 'clank' or 'dirge'.
///
/// A further optimization is that the search is done in parallel. We basically split the search
/// space into chunks, with the first chunk consisting of all solutions which begin with any of the first 32
/// words, the second chunk consisting of solutions which begin with any of the next 32 words etc.
///


/// A helper datatype which represents all anagrams of a single word
/// without repeated letters. Each bit position represents one letter,
/// so the word 'a' has the first bit set, the word 'ab' has the first and second, etc.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct WordBitmap(u32);
impl WordBitmap {
    /// Check if the letters of this instance overlap the other
    fn overlaps(self, other: WordBitmap) -> bool {
        self.0 & other.0 != 0
    }
    /// Remove all letters in 'other' from self
    fn remove(&mut self, other: WordBitmap) {
        self.0 &= !other.0;
    }
}

/// Pretty print a 'word'
impl Debug for WordBitmap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut t = self.0;
        write!(f, "Word( ").unwrap();
        while t != 0 {
            let digit = t.trailing_zeros();
            write!(
                f,
                "{} ",
                char::from_u32(b'a' as u32 + digit as u32).unwrap()
            )
            .unwrap();
            t &= !(1 << digit);
        }
        write!(f, ")")
    }
}
/// Allow x|=y for adding all set bits of rhs to self
impl BitOrAssign<WordBitmap> for WordBitmap {
    fn bitor_assign(&mut self, rhs: WordBitmap) {
        self.0 |= rhs.0;
    }
}

/// Given a string, return its word bitmap value. Words with repeated characters, or
/// with other than 5 characters, are invalid, and return None.
fn get_valid_word(s: &str) -> Option<WordBitmap> {
    let mut bitmap = 0u32;
    for c in s.bytes() {
        debug_assert!(c >= b'a' && c <= b'z');
        let letter_number = c - b'a';
        let mask = 1u32 << letter_number;
        if mask & bitmap != 0 {
            return None;
        }
        bitmap |= mask;
    }
    if bitmap.count_ones() != 5 {
        return None;
    }
    if bitmap == 0 {
        return None;
    }
    Some(WordBitmap(bitmap))
}


fn main() -> Result<()> {
    let all_words = std::fs::read_to_string("words_alpha.txt")?;

    // We gather all solutions from all threads in this list
    let all_solutions = Mutex::new(Vec::new());

    let mut word_list = vec![];
    let mut all_anagrams_of_word = HashMap::new();
    for line in all_words.split('\n') {
        if let Some(word) = get_valid_word(line) {
            all_anagrams_of_word.entry(word).or_insert_with( ||
                {
                    // Add words which haven't yet been added, and for which no anagram has been added.
                    word_list.push(word);
                    Vec::new()
                }).push(line);
        }
    }

    // Sort the word list in "bitmap-order"
    word_list.sort();

    const SEARCH_SPACE: usize = 1usize << 26;

    // Maintain a list of states which we know yield no solutions.
    // This is a bitmap with 2^26 bits.
    let mut dead_ends = vec![];
    dead_ends.resize_with(SEARCH_SPACE,||AtomicU64::new(0));

    // Iterate over all batches (size 32) in parallel
    const BATCH_SIZE:usize = 32;
    (0..(word_list.len()+BATCH_SIZE-1)/BATCH_SIZE).into_par_iter().for_each(|i|{
        calculate_solution_in_thread(
            i*BATCH_SIZE..((i+1)*BATCH_SIZE),
            &word_list,
            &all_anagrams_of_word,
            &dead_ends,
            |sol|{
                let mut guard = all_solutions.lock().unwrap();
                guard.push(sol.to_string());
            }).unwrap();
    });

    // Write all solutions to file solutions.txt
    let mut solutions_outputfile = std::fs::File::create("solutions.txt").unwrap();
    for sol in all_solutions.lock().unwrap().iter() {
        writeln!(solutions_outputfile, "{}", sol).unwrap();
    }
    println!(
        "Search complete. Found {} solutions. See file solutions.txt.",
        all_solutions.lock().unwrap().len()
    );
    Ok(())
}

// A helper which finds all words the could still be part of the solution,
// given that the letters 'used_letters' have already been spent.
// The cache is used to avoid recalculating the exact same values.
#[inline(always)]
fn get_possible_words<'a>(
    cache: &'a mut HashMap<WordBitmap, Vec<WordBitmap>>,
    used_letters: WordBitmap,
    all_words: &[WordBitmap],
) -> &'a Vec<WordBitmap> {
    cache
        .entry(used_letters)
        .or_insert_with(move || {
            let mut temp = vec![];
            for word in all_words {
                if !used_letters.overlaps(*word) {
                    temp.push(*word);
                }
            }

            temp
        })

}

/// Run in each thread, finds all solutions which begin with words in 'first_word_range' within 'word_list'.
fn calculate_solution_in_thread<'a>(first_word_range: Range<usize>, word_list: &Vec<WordBitmap>,
                                    all_anagrams_of_word:&HashMap<WordBitmap,Vec<&'a str>>,
                                    dead_ends: &Vec<AtomicU64>,
                                    mut print_solution: impl FnMut(&str)) -> Result<()> {

    let num_words = word_list.len();

    let mut next_to_explore_for_level = vec![first_word_range.start, 0usize, 0usize, 0usize, 0usize];

    let mut cur_used_letters = WordBitmap(0);
    let mut depth = 0;


    let mut available_words_for_letters: HashMap<WordBitmap, Vec<WordBitmap>> = HashMap::new();


    let mut dead_ends_candidates = IndexSet::new();


    fn is_dead_end(word: WordBitmap, bitmap: &[AtomicU64]) -> bool {
        let index = word.0 as u64;
        let u64index = index / 64;
        let bitoffset = index % 64;
        bitmap[u64index as usize].load(Ordering::Relaxed) & (1u64 << bitoffset) != 0
    }
    fn mark_dead_end(word: WordBitmap, bitmap: &[AtomicU64]) {
        let index = word.0 as u64;
        let u64index = index / 64;
        let bitoffset = index % 64;
        bitmap[u64index as usize].fetch_or(1u64 << bitoffset, Ordering::Relaxed);
    }

    let mut word_stack = [
        WordBitmap(0xffff_ffff),
        WordBitmap(0),
        WordBitmap(0),
        WordBitmap(0),
        WordBitmap(0),
        WordBitmap(0),
    ];

    let mut no_solutions_for_current_first_word = true;
    'outer: loop {
        let curwordlist = get_possible_words(
            &mut available_words_for_letters,
            cur_used_letters,
            &word_list,
        );


        let limit;
        if depth>0 {
            limit = num_words;
        } else {
            limit = first_word_range.end-next_to_explore_for_level[depth]; // Only iterate through as many first words as are designated for this thread
        }
        for (index, word) in curwordlist
            .iter()
            .copied()
            .enumerate()
            .skip(next_to_explore_for_level[depth])
            .take(limit)
        {
            if word < word_stack[depth] {
                let mut cand_cur_used_letters = cur_used_letters;
                cand_cur_used_letters |= word;

                if is_dead_end(cand_cur_used_letters, &dead_ends) {
                    continue;
                }

                word_stack[depth + 1] = word;

                if depth + 1 == 5 {
                    //Found solution!

                    no_solutions_for_current_first_word = false;
                    let mut variations: Vec<Vec<&str>> = vec![vec![]];
                    for word in word_stack[1..].iter().copied() {
                        let next = take(&mut variations);
                        for anagram in &all_anagrams_of_word[&word] {
                            for item in next.iter() {
                                let mut temp = item.clone();
                                temp.push(anagram);
                                variations.push(temp);
                            }
                        }
                    }
                    for variation in &variations {
                        let mut solprint = "".to_string();
                        for word in variation {
                            solprint+=&format!("{:?}\t", word);
                        }
                        print_solution(&solprint);
                    }
                    continue;
                }
                cur_used_letters = cand_cur_used_letters;
                next_to_explore_for_level[depth] = index + 1;
                depth += 1;

                next_to_explore_for_level[depth] = 0;
                continue 'outer;
            } else {
                break;
            }
        }

        if no_solutions_for_current_first_word {
            dead_ends_candidates.insert(cur_used_letters);
        }

        if depth == 0 {
            break 'outer;
        }
        depth -= 1;

        let cur_top_word = word_stack[depth + 1];

        cur_used_letters.remove(cur_top_word);

        if depth == 0 {
            if no_solutions_for_current_first_word {
                for cand in dead_ends_candidates.drain(..) {
                    mark_dead_end(cand, & dead_ends);
                }
            }
            dead_ends_candidates.clear();
            no_solutions_for_current_first_word = true;
        }
    }
    Ok(())
}
