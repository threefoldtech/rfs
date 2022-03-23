# How to use
`./read_time -bsize <block_size> -repeat <read_count> -lpath <log_file_path> -fpath <file_path_to_read>`

### Options
 - **bsize**: unsigned interger number in bytes which represent the read chunk (Default: 100)
 - **repeat**: unsigned integer number represents the number of file read [read repetition] (Default: 1)
 - **lpath**: the output result will be logged here (Default: /tmp/read_log)
 - **fpath**: the file under test which will be read (Default: . [the same path of this binary])

# Example:

```
$ ls 
read_time video_file.mp4

$ ./read_time -bsize 1024 -repeat 2 -lpath /tmp/o_log -fpath ./video_file.mp4
Read time (all blocks): 92039.000 us  (92.04 ms)
Avrg time (for the blocks): 0.919 us

Read time (all blocks): 90596.000 us  (90.60 ms)
Avrg time (for the blocks): 0.904 us

======[2 times the file read | 1024 Bytes block size]=========
Average read time : 95173 us (95.17 ms)
Min read time: 90596 us (90.60 ms)
Max read time: 106284 us (106.28 ms)

```