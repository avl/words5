extern crate anyhow;

use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::ops::BitOrAssign;
use anyhow::Result;
use std::io::Write;
#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash)]
struct WordBitmap(u32);
impl WordBitmap {
    fn overlaps(self, other: WordBitmap) -> bool {
        self.0 & other.0 != 0
    }
    fn remove(&mut self, other: WordBitmap) {
        self.0 &= !other.0;
    }
    fn has_used_all_letters(self) -> bool {
        self.0 == (1<<25)-1
    }
    fn letters_used(self) -> u32 {
        self.0.count_ones()
    }
}
impl Debug for WordBitmap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut t = self.0;
        write!(f,"Word( ").unwrap();
        while t!=0 {
            let digit = t.trailing_zeros();
            write!(f,"{} ",char::from_u32('a' as u32 + digit as u32).unwrap()).unwrap();
            t&=!(1<<digit);
        }
        write!(f,")")
    }
}
impl BitOrAssign<WordBitmap> for WordBitmap {
    fn bitor_assign(&mut self, rhs: WordBitmap) {
        self.0|=rhs.0;
    }
}
fn main() -> Result<()> {
    let all_words = std::fs::read_to_string("words.txt")?;


    /// Given a string, return its word bitmap value. Words with repeated characters
    /// are not valid, and return None.
    fn get_valid_word(s:&str) -> Option<WordBitmap> {
        let mut bitmap=0u32;
        for c in s.bytes() {
            debug_assert!(c>='a' as u8 && c<='z' as u8);
            let letter_number = c - 'a' as u8;
            let mask = 1u32<<letter_number;
            if mask & bitmap != 0 {
                return None;
            }
            bitmap |= mask;
        }
        if bitmap==0 {
            return None;
        }
        Some(WordBitmap(bitmap))
    }
    let mut candidate_words = HashMap::new();
    let mut word_list = vec![];
    let mut anagrams = HashMap::new();
    for line in all_words.split('\n') {
        if let Some(word) = get_valid_word(line) {
            anagrams.entry(word).or_insert_with(||vec![]).push(line);
            candidate_words.entry(word).or_insert_with(||{
                word_list.push(word);
                line
            });
        }
    }



    let word_list = word_list;

    let num_words = word_list.len();

    let mut dead_ends = HashSet::new();
    println!("Num words: {}",num_words);
    let mut explored = vec![0usize,0usize,0usize,0usize,0usize];
    let mut cur_used_letters = WordBitmap(0);
    let mut depth = 0;
    let mut solutions_found = 0;
    let mut dead_ends_eliminated = 0;
    let mut solprint = std::fs::File::create("solutions.txt").unwrap();
    'outer: loop {
        if depth <= 1 {
            dbg!(&explored,&cur_used_letters,&depth,dead_ends_eliminated,solutions_found);
            println!("--------------");
        }
        for (index,word) in word_list.iter().copied().enumerate().skip(explored[depth]+1) {
            if !cur_used_letters.overlaps(word) {
                let mut cand_cur_used_letters = cur_used_letters;
                cand_cur_used_letters|=word;

                if dead_ends.contains(&cand_cur_used_letters){
                    //println!("Visited dead-end at depth {}: {:?}", depth,cand_cur_used_letters);
                    dead_ends_eliminated+=1;
                    continue; //this is a dead end, don't explore further
                }

                cur_used_letters=cand_cur_used_letters;
                explored[depth]=index;
                depth+=1;
                //println!("Chose word {:?}",word);
                if depth==5 {
                    if cur_used_letters.letters_used()==25 {
                        println!("Found solution: (using letters: {:?})", cur_used_letters);
                        for exp in explored.iter().copied() {
                            write!(solprint,"{:?} ", anagrams[&word_list[exp]]);
                        }
                        writeln!(solprint,"");
                        solutions_found+=1;
                        //break 'outer;
                    }
                    depth -= 1;
                    cur_used_letters.remove(word);
                    continue;
                }
                explored[depth]=index;
                continue 'outer;
            }
        }

        // If we get here, we found no other word which could be used.
        // We now know that given 'cur_used_letters', there is no solution.
        // We insert this to the dead-end set
        // We must back up the search tree.
        //println!("Marking dead-end: {:?}", cur_used_letters);
        dead_ends.insert(cur_used_letters);
        if depth == 0 {
            println!("Search complete. Found {} solutions", solutions_found);
            break 'outer;
        }
        depth-=1;
        let unusable_word = explored[depth];
        //println!("Unchose word {:?}",word_list[unusable_word]);
        cur_used_letters.remove(word_list[unusable_word]);





    }
    Ok(())
}
