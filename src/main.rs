extern crate anyhow;
extern crate indexmap;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::ops::BitOrAssign;
use anyhow::Result;
use std::io::Write;
use std::mem::take;
use std::rc::Rc;
use indexmap::IndexSet;

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



    word_list.sort();
    let word_list = word_list;

    let num_words = word_list.len();

    //let mut dead_ends:HashSet<WordBitmap> = HashSet::new();
    println!("Num words: {}",num_words);
    let mut explored = vec![0usize,0usize,0usize,0usize,0usize];
    let mut had_solution = 0u32;
    let mut cur_used_letters = WordBitmap(0);
    let mut depth = 0;
    let mut solutions_found = 0;
    //let mut dead_ends_eliminated = 0;
    let mut solprint = std::fs::File::create("solutions.txt").unwrap();

    let mut available_words_for_letters:HashMap<WordBitmap,Rc<Vec<WordBitmap>>> = HashMap::new();
    /*
    for used_letter_set in 0u32..(1<<26) {
        if used_letter_set%(1<<20)==0 {
            println!("{} / {}", used_letter_set, 1<<26);
        }

        let used_letter_set = WordBitmap(used_letter_set);
        let num_letters = used_letter_set.letters_used();
        if num_letters>=15 && num_letters%5==0 {
            //let unused_letters = ((!c)&((1<<26)-1));
            let mut sublist = available_words_for_letters.entry(used_letter_set).or_insert_with(||Rc::new(Vec::new()));
            for word in word_list.iter() {
                if word.overlaps(used_letter_set)==false {
                    sublist.push(*word);
                }
            }
        }
    }
*/
    fn get_possible_words<'a>(cache: &'a mut HashMap<WordBitmap,Rc<Vec<WordBitmap>>>, used_letters: WordBitmap, all_words: &[WordBitmap]) -> Rc<Vec<WordBitmap>> {
        cache.entry(used_letters).or_insert_with(move||{
            let mut temp = vec![];
            for word in all_words {
                if !used_letters.overlaps(*word)
                {
                    temp.push(*word);
                }
            }

            Rc::new(temp)
        }).clone()
    }

    println!("Precalc done!");

    let mut used_letters_stack = [WordBitmap(0),WordBitmap(0),WordBitmap(0),WordBitmap(0),WordBitmap(0)];

    let mut dead_ends_candidates = IndexSet::new();
    let mut dead_ends = HashSet::new();
    let mut word_stack = [WordBitmap(0xffff_ffff),WordBitmap(0),WordBitmap(0),WordBitmap(0),WordBitmap(0),WordBitmap(0)];

    let mut solutionless = true;
    'outer: loop {
        let mut dbg = false;
        if depth <= 0 && explored[0]%100==0/* || explored[0]>=5976*/{
            dbg!(&explored,&cur_used_letters,&depth,solutions_found,dead_ends.len());
            println!("--------------");
            dbg = true;
        }

        //for (index,word) in word_list.iter().copied().enumerate().skip(explored[depth]) {
        let curwordlist = get_possible_words(&mut available_words_for_letters, cur_used_letters,&word_list);
        used_letters_stack[depth] = cur_used_letters;

        if dbg {
            println!("Given {:?} letters used, words are: {:?}. Indexing at {} ", cur_used_letters, curwordlist.len(), explored[depth]);
        }
        //
        for (index,word) in curwordlist.iter().copied().enumerate().skip(explored[depth]){//.iter().copied().enumerate().skip(explored[depth]) {
            if word<word_stack[depth] {

                let mut cand_cur_used_letters = cur_used_letters;
                cand_cur_used_letters|=word;

                if dead_ends.contains(&cand_cur_used_letters) {
                    continue;
                }

                /*if dead_ends.contains(&cand_cur_used_letters){
                    //println!("Visited dead-end at depth {}: {:?}", depth,cand_cur_used_letters);
                    dead_ends_eliminated+=1;
                    continue; //this is a dead end, don't explore further
                }*/

                cur_used_letters=cand_cur_used_letters;
                explored[depth]=index+1;
                word_stack[depth+1] = word;
                depth+=1;
                //println!("Chose word {:?}",word);
                if depth==5 {

                    solutionless = false;
                    //println!("Found solution: (using letters: {:?})", cur_used_letters);
                    let mut variations:Vec<Vec<&str>> = vec![vec![]];
                    for word in word_stack.iter().skip(1).copied() {
                        let mut next = take(&mut variations);
                        for anagram in &anagrams[&word] {
                            for item in next.iter() {
                                let mut temp=item.clone();
                                temp.push(anagram);
                                variations.push(temp);
                            }
                        }
                    }
                    for variation in &variations {
                        for word in variation {
                            write!(solprint,"{:?}\t", word).unwrap();
                        }
                        writeln!(solprint,"").unwrap();
                    }
                    solutions_found += variations.len();
                    had_solution|=31;
                    //break 'outer;

                    depth -= 1;
                    cur_used_letters.remove(word);
                    continue;
                }

                if dbg {
                    println!("Dive deeper");
                }

                explored[depth]=0;//index+1;
                continue 'outer;
            } else {
                break;
            }
        }

        // If we get here, we found no other word which could be used.
        // We now know that given 'cur_used_letters', there is no solution.
        // We insert this to the dead-end set
        // We must back up the search tree.
        //println!("Marking dead-end: {:?}", cur_used_letters);
        if had_solution==0
        {
            //dead_ends.insert(cur_used_letters);
        }
        if dbg {
            println!("Fell through on level {}", depth);
        }

        if solutionless {
            dead_ends_candidates.insert(cur_used_letters);
        }

        if depth == 0 {
            println!("Search complete. Found {} solutions", solutions_found);
            break 'outer;
        }
        had_solution&=!(1<<depth);
        depth-=1;
        let unusable_word = explored[depth]-1;

        let curwordlist = get_possible_words(&mut available_words_for_letters, used_letters_stack[depth], &word_list);
        cur_used_letters.remove(curwordlist[unusable_word]);
        if depth==0 {
            if solutionless {
                dead_ends.extend(take(&mut dead_ends_candidates));
            }
            dead_ends_candidates.clear();
            solutionless = true;
        }



    }
    Ok(())
}
