extern crate anyhow;
extern crate indexmap;
use anyhow::Result;
use indexmap::IndexSet;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io::Write;
use std::mem::take;
use std::ops::BitOrAssign;
use std::rc::Rc;


/// The basic idea of this program is:
///
/// Loop through all combinations of 5-letter words.
/// Aggressively prune the search tree by
/// 1) Symmetry-breaking: Require words to be in a specific order (all different orderings of the same 5 words are considered to be the same solution)
/// 2) Detect when a set of 'remaining' letters do not yield a solution
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
/// Allow x|=y for adding all set bits of y to x
impl BitOrAssign<WordBitmap> for WordBitmap {
    fn bitor_assign(&mut self, rhs: WordBitmap) {
        self.0 |= rhs.0;
    }
}

/// Given a string, return its word bitmap value. Words with repeated characters
/// are not valid, and return None.
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
    let all_words = std::fs::read_to_string("words.txt")?;



    let mut word_list = vec![];
    let mut all_anagrams_of_word = HashMap::new();
    for line in all_words.split('\n') {
        if let Some(word) = get_valid_word(line) {
            all_anagrams_of_word.entry(word).or_insert_with( ||
                                                     {
                                                         word_list.push(word);
                                                         Vec::new()
                                                     }).push(line);
        }
    }

    word_list.sort();
    let word_list = word_list;

    let num_words = word_list.len();

    let mut next_to_explore_for_level = vec![0usize, 0usize, 0usize, 0usize, 0usize];

    let mut cur_used_letters = WordBitmap(0);
    let mut depth = 0;
    let mut solutions_found = 0;

    let mut solprint = std::fs::File::create("solutions.txt").unwrap();

    let mut available_words_for_letters: HashMap<WordBitmap, Rc<Vec<WordBitmap>>> = HashMap::new();

    #[inline(always)]
    fn get_possible_words(
        cache: &mut HashMap<WordBitmap, Rc<Vec<WordBitmap>>>,
        used_letters: WordBitmap,
        all_words: &[WordBitmap],
    ) -> Rc<Vec<WordBitmap>> {
        cache
            .entry(used_letters)
            .or_insert_with(move || {
                let mut temp = vec![];
                for word in all_words {
                    if !used_letters.overlaps(*word) {
                        temp.push(*word);
                    }
                }

                Rc::new(temp)
            })
            .clone()
    }

    const SEARCH_SPACE: usize = 1usize << 26;

    let mut dead_ends_candidates = IndexSet::new();
    let mut dead_ends = vec![0u64; SEARCH_SPACE]; //HashSet::new();

    fn is_dead_end(word: WordBitmap, bitmap: &[u64]) -> bool {
        let index = word.0 as u64;
        let u64index = index / 64;
        let bitoffset = index % 64;
        bitmap[u64index as usize] & (1u64 << bitoffset) != 0
    }
    fn mark_dead_end(word: WordBitmap, bitmap: &mut [u64]) {
        let index = word.0 as u64;
        let u64index = index / 64;
        let bitoffset = index % 64;
        bitmap[u64index as usize] |= 1u64 << bitoffset;
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
        if depth == 0 && next_to_explore_for_level[0] % 100 == 0 {
            println!(
                "Searching: {:.2}%",
                next_to_explore_for_level[0] * 100 / num_words
            );
        }

        let curwordlist = &*get_possible_words(
            &mut available_words_for_letters,
            cur_used_letters,
            &word_list,
        );

        for (index, word) in curwordlist
            .iter()
            .copied()
            .enumerate()
            .skip(next_to_explore_for_level[depth])
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
                        for word in variation {
                            write!(solprint, "{:?}\t", word).unwrap();
                        }
                        writeln!(solprint).unwrap();
                    }
                    solutions_found += variations.len();

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
            println!(
                "Search complete. Found {} solutions. See file solutions.txt.",
                solutions_found
            );
            break 'outer;
        }
        depth -= 1;

        let cur_top_word = word_stack[depth + 1];
        cur_used_letters.remove(cur_top_word);

        if depth == 0 {
            if no_solutions_for_current_first_word {
                for cand in dead_ends_candidates.drain(..) {
                    mark_dead_end(cand, &mut dead_ends);
                }
            }
            dead_ends_candidates.clear();
            no_solutions_for_current_first_word = true;
        }
    }
    Ok(())
}
