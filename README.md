# Five words

This project finds five words with 5 letters each, with each letter of the alphabet used at most once.

This project is inspired by this fantastic Matt Parker-video: https://www.youtube.com/watch?v=_-AfhLQfb6w&t=947 .
If you haven't discovered Matt's channel, I very much recommend it. It's called 'Stand-up Maths': ( https://www.youtube.com/user/standupmaths/ ). 

This project is an optimization of the program created by Matt.

To use it:

1: Make sure to have the compiler for the programming language 'Rust' installed.

2: download the file 'words_alpha.txt' from https://github.com/dwyl/english-words .

3: Run the project like this ```time RUSTFLAGS='-C target-cpu=native' cargo run```

On my computer, the runtime varies, but the fastest runs complete in under one second (on an AMD 5950X).

```

Search complete. Found 831 solutions. See file solutions.txt.

real	0m0,789s
user	0m7,436s
sys	0m0,539s


```



