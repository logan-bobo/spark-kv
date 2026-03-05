# spark-kv
A simple persistent key value store, with an in memory index, implemented in Rust.

### Goals of this project
The goal of this project is to improve my [system programing](https://en.wikipedia.org/wiki/Systems_programming) knowledge through building a networked key-value database, with multithreading and asynchronous I/O[^1]. 

[^1]: Most of the learnings for this were derived from [pingcap/talent-plan](https://github.com/pingcap/talent-plan) it's a seriously good resource so check it out!
[^2]: A lot of the designs of this project comes from the [Bitcask paper](https://riak.com/assets/bitcask-intro.pdf)


### high level sequence 
```mermaid
sequenceDiagram
  participant cli as cli
  participant server as server
  participant disk as disk

  cli ->> server: startup kv server
  server ->> disk: replay WAL to build in memory index
  cli ->> server: set key hello with value world
  server ->> disk: persists key and value to WAL and updates index
  server ->> cli: exit 0
  cli ->> server: get key hello
  server ->> server: Look up file pointer in memory
  server ->> disk: read line from WAL containing value
  server ->> cli: return value for given key, exit 0
  cli ->> server: rm key hello
  server ->> disk: write rm command to disk
  server ->> server: remove key from in memory index
  server ->> cli: exit 0
```
