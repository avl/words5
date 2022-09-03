extern crate anyhow;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::ops::BitOrAssign;
use anyhow::Result;
use std::io::Write;
use std::mem::take;

#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash)]
struct WordBitmap(u32);
impl WordBitmap {
    fn overlaps(self, other: WordBitmap) -> bool {
        self.0 & other.0 != 0
    }
    fn remove(&mut self, other: WordBitmap) {
        self.0 &= !other.0;
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
        if bitmap.count_ones()!=5 {
            return None;
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

    let mut dead_ends:HashSet<WordBitmap> = HashSet::new();
    println!("Num words: {}",num_words);
    let mut explored = vec![0usize,0usize,0usize,0usize,0usize];
    let mut had_solution = 0u32;
    let mut cur_used_letters = WordBitmap(0);
    let mut depth = 0;
    let mut solutions_found = 0;
    let mut dead_ends_eliminated = 0;
    let mut solprint = std::fs::File::create("solutions.txt").unwrap();


    'outer: loop {
        if depth <= 0 {
            dbg!(&explored,&cur_used_letters,&depth,dead_ends_eliminated,solutions_found, dead_ends.len());
            println!("--------------");
        }

        for (index,word) in word_list.iter().copied().enumerate().skip(explored[depth]) {
            if !cur_used_letters.overlaps(word) {

                let mut cand_cur_used_letters = cur_used_letters;
                cand_cur_used_letters|=word;

                if dead_ends.contains(&cand_cur_used_letters){
                    //println!("Visited dead-end at depth {}: {:?}", depth,cand_cur_used_letters);
                    dead_ends_eliminated+=1;
                    continue; //this is a dead end, don't explore further
                }

                cur_used_letters=cand_cur_used_letters;
                explored[depth]=index+1;
                depth+=1;
                //println!("Chose word {:?}",word);
                if depth==5 {
                    if cur_used_letters.letters_used()==25 {
                        //println!("Found solution: (using letters: {:?})", cur_used_letters);
                        let mut variations:Vec<Vec<&str>> = vec![vec![]];
                        for exp in explored.iter().copied() {
                            let mut next = take(&mut variations);
                            for anagram in &anagrams[&word_list[exp-1]] {
                                for item in next.iter() {
                                    let mut temp=item.clone();
                                    temp.push(anagram);
                                    variations.push(temp);
                                }
                            }
                        }
                        for variation in &variations {
                            for word in variation {
                                write!(solprint,"{:?}\t", word);
                            }
                            writeln!(solprint,"");
                        }
                        solutions_found += variations.len();
                        had_solution|=31;
                        //break 'outer;
                    }

                    depth -= 1;
                    cur_used_letters.remove(word);
                    continue;
                }


                explored[depth]=index+1;
                continue 'outer;
            }
        }

        // If we get here, we found no other word which could be used.
        // We now know that given 'cur_used_letters', there is no solution.
        // We insert this to the dead-end set
        // We must back up the search tree.
        //println!("Marking dead-end: {:?}", cur_used_letters);
        if had_solution==0
        {
            dead_ends.insert(cur_used_letters);
        }

        if depth == 0 {
            println!("Search complete. Found {} solutions", solutions_found);
            break 'outer;
        }
        had_solution&=!(1<<depth);
        depth-=1;
        let unusable_word = explored[depth]-1;
        //println!("Unchose word {:?}",word_list[unusable_word]);
        cur_used_letters.remove(word_list[unusable_word]);




    }
    Ok(())
}
