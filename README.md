# spark-kv
A simple persistent key value store, with an in memory index, implemented in Rust.

# Goals of this project
The goal of this project is to improve my [system programing](https://en.wikipedia.org/wiki/Systems_programming) knowledge through building a networked key-value database, with multithreading and asynchronous I/O. Exploring...

- Serialization with [serde]
- Simple log-structured storage, inspired by [bitcask]
- Network programming with std and [tokio]
- Benchmarking with [criterion]
- Parallel programming with [crossbeam] and more
- Asyncronous programming with Rust [futures]
